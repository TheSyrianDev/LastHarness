//! Model types: a role asks for a kind of model, and the type says which model that is today.
//! The three stock types (frontier, mid-tier, fastest) start without a model; nothing is ranked or
//! assigned for the user. Any type, stock ones included, can be edited or deleted.

use std::collections::BTreeMap;

use lh_learn::valid_name;
use lh_store::Store;

use crate::roles::{RoleBook, Target};
use crate::settings::UnavailablePolicy;
use crate::{Error, Result};

#[derive(Clone, Debug, PartialEq)]
pub struct ModelType {
    pub name: String,
    pub model: Option<String>,
    /// Tried in order when the model is unavailable and the setting says to fall back.
    pub fallbacks: Vec<String>,
    pub stock: bool,
}

/// The model to use, and a note for the user when it is not the one first asked for.
#[derive(Clone, Debug, PartialEq)]
pub struct Resolution {
    pub model: String,
    pub note: Option<String>,
}

pub struct ModelTypes<'a> {
    store: &'a Store,
}

const COLS: &str = "name, model, fallbacks, stock";

fn from_row(r: &lh_store::rusqlite::Row) -> lh_store::rusqlite::Result<ModelType> {
    Ok(ModelType { name: r.get(0)?, model: r.get(1)?, fallbacks: serde_json::from_str(&r.get::<_, String>(2)?).unwrap_or_default(), stock: r.get::<_, i64>(3)? != 0 })
}

impl<'a> ModelTypes<'a> {
    pub fn new(store: &'a Store) -> Self {
        Self { store }
    }

    pub fn list(&self) -> Result<Vec<ModelType>> {
        Ok(self.store.with(|c| {
            let mut q = c.prepare(&format!("select {COLS} from model_types order by position, name"))?;
            let rows = q.query_map([], from_row)?;
            rows.collect()
        })?)
    }

    pub fn get(&self, name: &str) -> Result<ModelType> {
        use lh_store::rusqlite::OptionalExtension;
        self.store.with(|c| c.query_row(&format!("select {COLS} from model_types where name = ?1"), [name], from_row).optional())?.ok_or_else(|| Error::NotFound(format!("model type {name}")))
    }

    fn log(&self, what: &str, name: &str) {
        let _ = self.store.log("model_type", "-", "user", &serde_json::json!({"change": what, "type": name}));
    }

    pub fn add(&self, name: &str, model: Option<&str>) -> Result<ModelType> {
        let name = name.trim().to_lowercase();
        if !valid_name(&name) {
            return Err(Error::Invalid("a type name uses lower case letters, digits and single hyphens".into()));
        }
        if self.get(&name).is_ok() {
            return Err(Error::Invalid(format!("there is already a type called {name}")));
        }
        self.store.with(|c| c.execute("insert into model_types(name, model, position) values (?1, ?2, (select coalesce(max(position), 0) + 1 from model_types))", (&name, model)).map(|_| ()))?;
        self.log("add", &name);
        self.get(&name)
    }

    pub fn set_model(&self, name: &str, model: Option<&str>) -> Result<ModelType> {
        self.get(name)?;
        self.store.with(|c| c.execute("update model_types set model = ?2 where name = ?1", (name, model.map(str::trim).filter(|m| !m.is_empty()))).map(|_| ()))?;
        self.log("model", name);
        self.get(name)
    }

    pub fn set_fallbacks(&self, name: &str, list: &[String]) -> Result<ModelType> {
        self.get(name)?;
        self.store.with(|c| c.execute("update model_types set fallbacks = ?2 where name = ?1", (name, serde_json::to_string(list).unwrap_or_default())).map(|_| ()))?;
        self.log("fallbacks", name);
        self.get(name)
    }

    /// Delete a type. Roles that use it must each be given another type or model in `reassign`, or the
    /// delete is refused and the roles are listed, so nothing is left pointing at a type that is gone.
    pub fn delete(&self, name: &str, roles: &RoleBook, reassign: &BTreeMap<String, Target>) -> Result<()> {
        self.get(name)?;
        let using = roles.using_type(name)?;
        let missing: Vec<String> = using.iter().filter(|r| !reassign.contains_key(*r)).cloned().collect();
        if !missing.is_empty() {
            return Err(Error::NeedsReassign(missing));
        }
        for (role, target) in reassign.iter().filter(|(r, _)| using.contains(r)) {
            if let Target::Type(t) = target {
                if t == name || self.get(t).is_err() {
                    return Err(Error::Invalid(format!("{role} cannot move to the type {t}: it does not exist (or is the one being deleted)")));
                }
            }
            let mut r = roles.get(role)?;
            r.target = target.clone();
            roles.save(&r, &format!("model type {name} deleted"))?;
        }
        self.store.with(|c| c.execute("delete from model_types where name = ?1", [name]).map(|_| ()))?;
        self.log("delete", name);
        Ok(())
    }

    /// Which model to use for a role's target. `available` is what the server lists (empty means the
    /// server cannot say, and everything is assumed to be there). `allowed` is the sensitive-data rule:
    /// a fallback it forbids is never used.
    pub fn resolve(&self, target: &Target, available: &[String], policy: UnavailablePolicy, allowed: &dyn Fn(&str) -> bool) -> Result<Resolution> {
        let here = |m: &str| available.is_empty() || available.iter().any(|a| a == m);
        match target {
            Target::Model(m) if here(m) => Ok(Resolution { model: m.clone(), note: None }),
            Target::Model(m) => Err(Error::Unavailable(format!("The model {m} is not available on the server. Pick another model for this role."))),
            Target::Type(t) => {
                let ty = self.get(t)?;
                let Some(model) = ty.model.clone() else {
                    return Err(Error::Unavailable(format!("The type {t} has no model yet. Choose one in the model settings.")));
                };
                if here(&model) {
                    return Ok(Resolution { model, note: None });
                }
                if policy == UnavailablePolicy::Fallback {
                    if let Some(f) = ty.fallbacks.iter().find(|f| here(f) && allowed(f)) {
                        return Ok(Resolution { model: f.clone(), note: Some(format!("{model} is not available, so {f} (the next in the list for {t}) is used.")) });
                    }
                    return Err(Error::Unavailable(format!("{model} (the {t} type) is not available and none of its fallbacks can be used. Pick another model now.")));
                }
                Err(Error::Unavailable(format!("{model} (the {t} type) is not available. Pick another model now, or give the type a fallback list.")))
            }
        }
    }
}

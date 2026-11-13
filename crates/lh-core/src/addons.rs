//! Local add-on hub, folder version. An add-on is a folder with an `addon.json` manifest
//! (`{"name": "...", "version": "..."}`). Installing copies it into the hub folder; removing deletes its
//! copy. Nothing is downloaded and nothing is run here.

use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
pub struct Installed {
    pub name: String,
    pub version: String,
}

/// Read the manifest of an add-on folder.
pub fn manifest(src: &Path) -> Result<Installed, String> {
    let text = fs::read_to_string(src.join("addon.json")).map_err(|e| format!("no addon.json in {}: {e}", src.display()))?;
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("addon.json is not JSON: {e}"))?;
    let name = v["name"].as_str().filter(|n| valid_name(n)).ok_or("addon.json needs a name made of letters, digits, - or _")?;
    let version = v["version"].as_str().unwrap_or("0.0.0");
    Ok(Installed { name: name.into(), version: version.into() })
}

fn valid_name(n: &str) -> bool {
    !n.is_empty() && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Copy the add-on folder `src` into `hub`. An add-on with the same name is replaced.
pub fn install(hub: &Path, src: &Path) -> Result<Installed, String> {
    let m = manifest(src)?;
    let dest = hub.join(&m.name);
    let _ = fs::remove_dir_all(&dest);
    copy_dir(src, &dest).map_err(|e| format!("could not copy {}: {e}", src.display()))?;
    Ok(m)
}

/// Remove an installed add-on by name. Returns false when it was not there.
pub fn remove(hub: &Path, name: &str) -> Result<bool, String> {
    if !valid_name(name) {
        return Err("bad add-on name".into());
    }
    let dest = hub.join(name);
    if !dest.is_dir() {
        return Ok(false);
    }
    fs::remove_dir_all(&dest).map_err(|e| e.to_string())?;
    Ok(true)
}

/// The installed add-ons in `hub`, sorted by name.
pub fn list(hub: &Path) -> Vec<Installed> {
    let mut out: Vec<Installed> = fs::read_dir(hub)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| manifest(&e.path()).ok())
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

fn copy_dir(src: &Path, dest: &PathBuf) -> std::io::Result<()> {
    fs::create_dir_all(dest)?;
    for e in fs::read_dir(src)? {
        let e = e?;
        let to = dest.join(e.file_name());
        if e.file_type()?.is_dir() {
            copy_dir(&e.path(), &to)?;
        } else {
            fs::copy(e.path(), to)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_list_and_remove_a_sample_add_on() {
        let t = tempfile::tempdir().unwrap();
        let src = t.path().join("hello-addon");
        fs::create_dir_all(src.join("lib")).unwrap();
        fs::write(src.join("addon.json"), r#"{"name":"hello","version":"1.2.0"}"#).unwrap();
        fs::write(src.join("lib/readme.txt"), "hi").unwrap();
        let hub = t.path().join("hub");
        assert_eq!(install(&hub, &src).unwrap(), Installed { name: "hello".into(), version: "1.2.0".into() });
        assert!(hub.join("hello/lib/readme.txt").is_file());
        assert_eq!(list(&hub), vec![Installed { name: "hello".into(), version: "1.2.0".into() }]);
        assert_eq!(remove(&hub, "hello"), Ok(true));
        assert_eq!(remove(&hub, "hello"), Ok(false));
        assert!(list(&hub).is_empty());
    }

    #[test]
    fn a_bad_manifest_is_refused() {
        let t = tempfile::tempdir().unwrap();
        fs::write(t.path().join("addon.json"), r#"{"name":"../evil"}"#).unwrap();
        assert!(manifest(t.path()).is_err());
        assert!(remove(t.path(), "../x").is_err());
    }
}

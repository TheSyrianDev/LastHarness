use serde::Deserialize;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Record only. Nothing is ever stopped or asked.
    None,
    /// Refuse a call that would pass a cap.
    Caps,
    /// Ask before an expensive call or one that passes a soft limit. The default.
    #[default]
    Estimate,
}

/// What one model costs, in US dollars per million tokens. `model` may use `*` as a wildcard.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Price {
    pub model: String,
    #[serde(default)]
    pub input: f64,
    #[serde(default)]
    pub output: f64,
    /// Price of cached input tokens; defaults to `input`.
    pub cached_input: Option<f64>,
    /// Image models: US dollars per generated image.
    pub image: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    pub mode: Mode,
    /// Estimate mode: ask when one call is expected to cost more than this.
    pub approve_over_usd: f64,
    pub per_request_cap_usd: Option<f64>,
    pub daily_cap_usd: Option<f64>,
    pub monthly_cap_usd: Option<f64>,
    #[serde(rename = "price")]
    pub prices: Vec<Price>,
}

impl Default for Config {
    fn default() -> Self {
        Self { mode: Mode::Estimate, approve_over_usd: 0.25, per_request_cap_usd: None, daily_cap_usd: None, monthly_cap_usd: None, prices: vec![] }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("cannot read the spend settings: {0}")]
pub struct ConfigError(String);

impl Config {
    pub fn from_toml(text: &str) -> Result<Self, ConfigError> {
        let c: Config = toml::from_str(text).map_err(|e| ConfigError(e.to_string()))?;
        for v in [Some(c.approve_over_usd), c.per_request_cap_usd, c.daily_cap_usd, c.monthly_cap_usd].into_iter().flatten() {
            if v < 0.0 || !v.is_finite() {
                return Err(ConfigError("limits must be zero or more".into()));
            }
        }
        Ok(c)
    }

    pub fn price_of(&self, model: &str) -> Option<&Price> {
        self.prices.iter().find(|p| glob(&p.model, model))
    }
}

/// `*` matches any run of characters. Case-insensitive.
pub fn glob(pattern: &str, text: &str) -> bool {
    let (p, t) = (pattern.to_lowercase(), text.to_lowercase());
    let parts: Vec<&str> = p.split('*').collect();
    if parts.len() == 1 {
        return p == t;
    }
    let (first, last) = (parts[0], parts[parts.len() - 1]);
    if !t.starts_with(first) || !t[first.len()..].ends_with(last) || t.len() < first.len() + last.len() {
        return false;
    }
    let mut rest = &t[first.len()..t.len() - last.len()];
    for mid in &parts[1..parts.len() - 1] {
        match rest.find(mid) {
            Some(i) => rest = &rest[i + mid.len()..],
            None => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcards() {
        assert!(glob("gpt-*", "GPT-4o"));
        assert!(glob("*sonnet*", "claude-sonnet-5-5"));
        assert!(glob("a*b*c", "aXbYc"));
        assert!(!glob("a*b*c", "aXcYb"));
        assert!(!glob("gpt-4", "gpt-4o"));
        assert!(!glob("ab*ba", "aba"));
    }

    #[test]
    fn parses_settings_and_applies_defaults() {
        let c = Config::from_toml("").unwrap();
        assert_eq!((c.mode, c.approve_over_usd), (Mode::Estimate, 0.25));
        let c = Config::from_toml("mode = \"caps\"\ndaily_cap_usd = 5\n[[price]]\nmodel = \"gpt-*\"\ninput = 2.5\noutput = 10\n").unwrap();
        assert_eq!((c.mode, c.daily_cap_usd), (Mode::Caps, Some(5.0)));
        assert_eq!(c.price_of("gpt-4o").unwrap().output, 10.0);
        assert!(c.price_of("claude").is_none());
        assert!(Config::from_toml("mode = \"sometimes\"").is_err());
        assert!(Config::from_toml("daily_cap_usd = -1").is_err());
    }
}

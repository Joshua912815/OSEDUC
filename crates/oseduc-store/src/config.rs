use std::{env, fmt};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DatabaseConfig {
    pub database_url: SecretDatabaseUrl,
    pub auto_migrate: bool,
    pub enable_admin_seed: bool,
}

impl DatabaseConfig {
    pub fn from_env() -> Result<Self, DatabaseConfigError> {
        Self::from_getter(|key| env::var(key).ok())
    }

    pub fn from_getter(get: impl Fn(&str) -> Option<String>) -> Result<Self, DatabaseConfigError> {
        let database_url = non_empty(get("OSEDUC_DATABASE_URL"))
            .map(SecretDatabaseUrl::new)
            .ok_or(DatabaseConfigError::MissingDatabaseUrl)?;
        let auto_migrate = parse_bool("OSEDUC_AUTO_MIGRATE", get("OSEDUC_AUTO_MIGRATE"))?;
        let enable_admin_seed =
            parse_bool("OSEDUC_ENABLE_ADMIN_SEED", get("OSEDUC_ENABLE_ADMIN_SEED"))?;

        Ok(Self {
            database_url,
            auto_migrate,
            enable_admin_seed,
        })
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct SecretDatabaseUrl(String);

impl SecretDatabaseUrl {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose_secret(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretDatabaseUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<redacted>")
    }
}

impl fmt::Display for SecretDatabaseUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<redacted>")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DatabaseConfigError {
    MissingDatabaseUrl,
    InvalidBoolean { key: &'static str, value: String },
}

impl fmt::Display for DatabaseConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingDatabaseUrl => {
                formatter.write_str("OSEDUC_DATABASE_URL is required for the knowledge store")
            }
            Self::InvalidBoolean { key, value } => {
                write!(formatter, "{key} must be true or false, got {value}")
            }
        }
    }
}

impl std::error::Error for DatabaseConfigError {}

fn non_empty(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn parse_bool(key: &'static str, value: Option<String>) -> Result<bool, DatabaseConfigError> {
    let Some(value) = non_empty(value) else {
        return Ok(false);
    };
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(DatabaseConfigError::InvalidBoolean { key, value }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn getter<'a>(entries: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        |key| {
            entries
                .iter()
                .find_map(|(entry_key, value)| (*entry_key == key).then(|| (*value).to_owned()))
        }
    }

    #[test]
    fn requires_database_url() {
        let error =
            DatabaseConfig::from_getter(getter(&[])).expect_err("database url should be required");

        assert_eq!(error, DatabaseConfigError::MissingDatabaseUrl);
    }

    #[test]
    fn parses_database_flags() {
        let config = DatabaseConfig::from_getter(getter(&[
            (
                "OSEDUC_DATABASE_URL",
                "postgres://oseduc:dev-password@127.0.0.1:5432/oseduc",
            ),
            ("OSEDUC_AUTO_MIGRATE", "true"),
            ("OSEDUC_ENABLE_ADMIN_SEED", "1"),
        ]))
        .expect("config should parse");

        assert!(config.auto_migrate);
        assert!(config.enable_admin_seed);
    }

    #[test]
    fn database_url_debug_and_display_are_redacted() {
        let url = SecretDatabaseUrl::new("postgres://oseduc:dev-password@127.0.0.1/oseduc");

        assert_eq!(format!("{url:?}"), "<redacted>");
        assert_eq!(url.to_string(), "<redacted>");
        assert!(!format!("{url:?}").contains("dev-password"));
    }
}

#![forbid(unsafe_code)]

mod config;
mod repository;
mod seed;

pub use config::{DatabaseConfig, DatabaseConfigError, SecretDatabaseUrl};
pub use seed::{KnowledgeSeed, KnowledgeSeedError, KnowledgeSeedSummary};

use sqlx::{postgres::PgPoolOptions, PgPool};

const MIGRATIONS: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[derive(Clone)]
pub struct PostgresStore {
    pool: PgPool,
}

impl PostgresStore {
    pub async fn connect(database_url: &SecretDatabaseUrl) -> Result<Self, StoreError> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url.expose_secret())
            .await
            .map_err(|error| StoreError::Database(error.to_string()))?;
        Ok(Self { pool })
    }

    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn run_migrations(&self) -> Result<(), StoreError> {
        MIGRATIONS
            .run(&self.pool)
            .await
            .map_err(|error| StoreError::Migration(error.to_string()))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StoreError {
    Database(String),
    InvalidInput(String),
    InvalidSeed(String),
    Migration(String),
    NotFound(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(_) => formatter.write_str("knowledge store database operation failed"),
            Self::InvalidInput(message) => {
                write!(formatter, "knowledge store input is invalid: {message}")
            }
            Self::InvalidSeed(_) => formatter.write_str("knowledge seed validation failed"),
            Self::Migration(_) => formatter.write_str("knowledge store migration failed"),
            Self::NotFound(id) => write!(formatter, "knowledge store item not found: {id}"),
        }
    }
}

impl std::error::Error for StoreError {}

pub fn crate_name() -> &'static str {
    "oseduc-store"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_crate_name() {
        assert_eq!(crate_name(), "oseduc-store");
    }

    #[test]
    fn store_errors_are_sanitized_for_display() {
        let error = StoreError::Database(
            "postgres://user:password@example.invalid/oseduc connection failed".to_owned(),
        );

        assert!(!error.to_string().contains("password"));
        assert_eq!(
            error.to_string(),
            "knowledge store database operation failed"
        );
    }
}

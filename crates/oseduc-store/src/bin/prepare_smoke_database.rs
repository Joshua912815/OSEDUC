#![forbid(unsafe_code)]

use std::{
    env,
    error::Error,
    thread,
    time::{Duration, Instant},
};

use sqlx::postgres::PgPoolOptions;
use url::Url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let database_url = env::var("OSEDUC_DATABASE_URL")
        .map_err(|_| "OSEDUC_DATABASE_URL is required to prepare the smoke database")?;
    let timeout = env::var("OSEDUC_WAIT_FOR_POSTGRES_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(60));

    let (maintenance_url, target_database) = maintenance_url_and_target_database(&database_url)?;
    let pool = connect_with_retry(maintenance_url.as_str(), timeout).await?;

    sqlx::query(
        "SELECT pg_terminate_backend(pid) \
         FROM pg_stat_activity \
         WHERE datname = $1 AND pid <> pg_backend_pid()",
    )
    .bind(&target_database)
    .execute(&pool)
    .await
    .map_err(|_| "failed to terminate existing smoke database sessions")?;

    sqlx::query(&format!(
        "DROP DATABASE IF EXISTS {}",
        quote_identifier(&target_database)
    ))
    .execute(&pool)
    .await
    .map_err(|_| "failed to drop the smoke database")?;

    sqlx::query(&format!(
        "CREATE DATABASE {}",
        quote_identifier(&target_database)
    ))
    .execute(&pool)
    .await
    .map_err(|_| "failed to create the smoke database")?;

    pool.close().await;
    println!("Prepared isolated smoke database: {target_database}");
    Ok(())
}

async fn connect_with_retry(database_url: &str, timeout: Duration) -> Result<sqlx::PgPool, String> {
    let deadline = Instant::now() + timeout;

    loop {
        match PgPoolOptions::new()
            .max_connections(1)
            .connect(database_url)
            .await
        {
            Ok(pool) => return Ok(pool),
            Err(_) if Instant::now() < deadline => {
                eprintln!("Postgres is not ready yet");
                thread::sleep(Duration::from_secs(1));
            }
            Err(_) => return Err("Postgres did not become ready".to_owned()),
        }
    }
}

fn maintenance_url_and_target_database(
    database_url: &str,
) -> Result<(Url, String), Box<dyn Error>> {
    let mut url = Url::parse(database_url).map_err(|_| "OSEDUC_DATABASE_URL is not a valid URL")?;
    match url.scheme() {
        "postgres" | "postgresql" => {}
        _ => return Err("OSEDUC_DATABASE_URL must use a Postgres URL scheme".into()),
    }

    let database = url.path().trim_start_matches('/').to_owned();
    if database.is_empty() {
        return Err("OSEDUC_DATABASE_URL must include a database name".into());
    }
    if database == "postgres" {
        return Err("refusing to reset the Postgres maintenance database".into());
    }
    if !database
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return Err("smoke database name may only contain ASCII letters, numbers, and '_'".into());
    }

    url.set_path("postgres");
    Ok((url, database))
}

fn quote_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_maintenance_url_without_exposing_password() {
        let (maintenance_url, target_database) =
            maintenance_url_and_target_database("postgres://oseduc:secret@127.0.0.1:5432/smoke_db")
                .expect("valid database URL");

        assert_eq!(target_database, "smoke_db");
        assert_eq!(maintenance_url.path(), "/postgres");
    }

    #[test]
    fn rejects_unsafe_database_names() {
        let error =
            maintenance_url_and_target_database("postgres://oseduc:secret@127.0.0.1:5432/smoke-db")
                .expect_err("database names with '-' should be rejected");

        assert!(error
            .to_string()
            .contains("ASCII letters, numbers, and '_'"));
    }

    #[test]
    fn quotes_postgres_identifiers() {
        assert_eq!(quote_identifier("smoke_db"), "\"smoke_db\"");
        assert_eq!(quote_identifier("smoke\"db"), "\"smoke\"\"db\"");
    }
}

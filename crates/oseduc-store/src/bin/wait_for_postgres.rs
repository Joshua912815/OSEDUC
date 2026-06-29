#![forbid(unsafe_code)]

use std::{
    env,
    error::Error,
    thread,
    time::{Duration, Instant},
};

use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let database_url = env::var("OSEDUC_DATABASE_URL")
        .map_err(|_| "OSEDUC_DATABASE_URL is required to wait for Postgres")?;
    let timeout = env::var("OSEDUC_WAIT_FOR_POSTGRES_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(60));
    let deadline = Instant::now() + timeout;

    loop {
        match PgPoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await
        {
            Ok(pool) => {
                pool.close().await;
                return Ok(());
            }
            Err(error) if Instant::now() < deadline => {
                eprintln!("Postgres is not ready yet: {error}");
                thread::sleep(Duration::from_secs(1));
            }
            Err(error) => {
                return Err(format!("Postgres did not become ready: {error}").into());
            }
        }
    }
}

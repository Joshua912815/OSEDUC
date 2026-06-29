use oseduc_api::{build_router, ApiConfig, AppState, PublicConfig};
use oseduc_llm::LlmGateway;
use oseduc_store::PostgresStore;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ApiConfig::from_env()?;
    let gateway = LlmGateway::from_config(config.llm.clone())?;
    let store = PostgresStore::connect(&config.database.database_url).await?;
    if config.database.auto_migrate {
        store.run_migrations().await?;
    }
    let state = AppState {
        gateway,
        public_config: PublicConfig::new(&config.llm, &config.database),
        knowledge: Arc::new(store),
        admin_seed_enabled: config.database.enable_admin_seed,
        admin_token: config.admin_token,
    };
    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;

    axum::serve(listener, build_router(state)).await?;
    Ok(())
}

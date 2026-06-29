use oseduc_api::{build_router, ApiConfig, AppState, PublicConfig};
use oseduc_llm::LlmGateway;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ApiConfig::from_env()?;
    let gateway = LlmGateway::from_config(config.llm.clone())?;
    let state = AppState {
        gateway,
        public_config: PublicConfig::from(&config.llm),
    };
    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;

    axum::serve(listener, build_router(state)).await?;
    Ok(())
}

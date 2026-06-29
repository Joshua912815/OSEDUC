#![forbid(unsafe_code)]

mod config;
mod router;

pub use config::{ApiConfig, ApiConfigError, PublicConfig};
pub use router::{build_router, AppState};

pub fn crate_name() -> &'static str {
    "oseduc-api"
}

#[cfg(test)]
mod tests {
    #[test]
    fn exposes_crate_name() {
        assert_eq!(super::crate_name(), "oseduc-api");
    }
}

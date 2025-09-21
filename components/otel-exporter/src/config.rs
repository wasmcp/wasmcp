use std::sync::OnceLock;
use crate::providers::grafana::{GrafanaConfig, GrafanaProvider};

static CACHED_CONFIG: OnceLock<Option<GrafanaConfig>> = OnceLock::new();

pub fn get_grafana_config() -> Option<&'static GrafanaConfig> {
    CACHED_CONFIG.get_or_init(|| {
        match GrafanaProvider::load_config_from_spin() {
            Ok(config) => Some(config),
            Err(e) => {
                println!("[CONFIG] Failed to load Grafana configuration: {:?}", e);
                None // User didn't provide OTEL configuration
            }
        }
    }).as_ref()
}

pub fn is_tracing_enabled() -> bool {
    get_grafana_config().is_some()
}
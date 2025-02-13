use prometheus::Registry;
use std::sync::Arc;
use tracing::debug;

#[derive(Clone)]
pub struct AppState {
    pub registry: Arc<Registry>,
}

impl AppState {
    pub fn new() -> (Self, Registry) {
        debug!("Created new prometheus registry");
        let registry = Registry::new();
        let state = Self {
            registry: Arc::new(registry.clone()),
        };
        (state, registry)
    }
}

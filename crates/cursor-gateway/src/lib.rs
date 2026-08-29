pub mod account;
pub mod actor;
pub mod agent;
pub mod analytics;
pub mod bidi_append;
pub mod chat;
pub mod connect;
pub mod error;
pub mod handlers;
pub mod harness;
pub mod inbox;
pub mod interaction;
pub mod lifecycle;
pub mod model;
pub mod model_catalog;
pub mod prompting;
pub mod proto;
pub mod provider;
pub mod proxy;
pub mod run_sse;
pub mod runtime;
pub mod server;
pub mod sessions;
pub mod web;

pub use error::{GatewayError, Result};
pub use harness::{
    clear_proxy_settings, ensure_local_ultra_account, ensure_local_ultra_account_at,
    force_inject_ultra, force_inject_ultra_at, inject_if_missing, inject_if_missing_at,
    settings_match, settings_path, state_db_path, write_proxy_settings, CaManager, CaState,
    LoadedCa, ProxyRuntime,
};
pub use handlers::build_router;
pub use runtime::CursorGatewayRuntime;
pub use server::start_backend_server;
pub use sessions::CursorSessionRegistry;

#[cfg(test)]
mod integration_tests;

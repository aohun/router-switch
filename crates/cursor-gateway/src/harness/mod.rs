pub mod account;
pub mod ca;
pub mod proxy;
pub mod settings;

pub use account::{
    ensure_local_ultra_account, ensure_local_ultra_account_at, force_inject_ultra,
    force_inject_ultra_at, inject_if_missing, inject_if_missing_at, state_db_path,
};
pub use ca::{CaManager, CaState, LoadedCa};
pub use proxy::{is_cursor_host, is_local_path, ProxyRuntime, UPSTREAM_URL_HEADER};
pub use settings::{
    clear_proxy_settings, clear_stale_managed_settings, read_settings, settings_match,
    settings_path, write_proxy_settings, write_settings,
};

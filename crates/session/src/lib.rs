//! App-layer glue: SQLite SSOT + live config adapters for Codex, Claude, Grok, OpenCode, Pi, Cursor, and ZCode.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use adapters_claude::{
    read_live as read_claude_live, resolve_claude_paths,
    write_live_for_provider as write_claude_live, ClaudeAdapterError, ClaudePaths,
};
use adapters_codex::{
    read_live as read_codex_live, resolve_codex_paths, write_live_for_provider as write_codex_live,
    CodexAdapterError, CodexPaths,
};
use adapters_grok::{
    read_live as read_grok_live, resolve_grok_paths, write_live_for_provider as write_grok_live,
    GrokAdapterError, GrokPaths,
};
use adapters_opencode::{
    resolve_opencode_paths, write_live_for_provider as write_opencode_live, OpenCodeAdapterError,
    OpenCodePaths,
};
use adapters_pi::{
    resolve_pi_paths, write_live_for_provider as write_pi_live, PiAdapterError, PiPaths,
};
use adapters_workbuddy::{
    delete_live_for_model as delete_workbuddy_live, read_live_models as read_workbuddy_live,
    resolve_workbuddy_paths, write_live_for_model as write_workbuddy_live, WorkBuddyAdapterError,
    WorkBuddyPaths,
};
use adapters_zcode::{
    delete_live_for_provider as delete_zcode_live, list_custom_providers_from_config,
    resolve_zcode_paths, write_live_for_provider as write_zcode_live, ZCodeAdapterError,
    ZCodePaths,
};
pub use cursor_gateway::{CaState, LoadedCa};
use cursor_gateway::{CursorGatewayRuntime, GatewayError};
use domain::{
    backfill_claude_settings, backfill_codex_settings, backfill_cursor_settings,
    backfill_grok_settings, inspect_all_tools, inspect_tool_environment, new_provider_id,
    parse_claude_form, parse_codex_form, parse_cursor_form, parse_grok_form, parse_opencode_form,
    parse_pi_form, parse_workbuddy_form, parse_zcode_form, AppKind, ClaudeForm, CodexForm,
    CursorForm, DomainError, GrokForm, OpenCodeForm, PiForm, Provider, ProviderForm,
    ProviderSettings, WorkBuddyForm, WorkBuddySettings, ZCodeForm, OFFICIAL_CLAUDE_ID,
    OFFICIAL_CODEX_ID, OFFICIAL_CURSOR_ID, OFFICIAL_GROK_ID, OFFICIAL_OPENCODE_ID, OFFICIAL_PI_ID,
    OFFICIAL_WORKBUDDY_ID, OFFICIAL_ZCODE_ID,
};
use parking_lot::Mutex;
use std::sync::OnceLock;
pub use store::{
    AppLanguage, AppSettings, LogConfig, LogLevel, Store, StoreError, ThemePreference,
};
use thiserror::Error;

static TOKIO_RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

pub fn tokio_runtime() -> &'static tokio::runtime::Runtime {
    TOKIO_RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to initialize tokio runtime for cursor gateway")
    })
}

#[derive(Debug, Error)]
pub enum SessionError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    CodexAdapter(#[from] CodexAdapterError),
    #[error(transparent)]
    ClaudeAdapter(#[from] ClaudeAdapterError),
    #[error(transparent)]
    GrokAdapter(#[from] GrokAdapterError),
    #[error(transparent)]
    OpenCodeAdapter(#[from] OpenCodeAdapterError),
    #[error(transparent)]
    PiAdapter(#[from] PiAdapterError),
    #[error(transparent)]
    CursorGateway(#[from] GatewayError),
    #[error(transparent)]
    ZCodeAdapter(#[from] ZCodeAdapterError),
    #[error(transparent)]
    WorkBuddyAdapter(#[from] WorkBuddyAdapterError),
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error("{0}")]
    Message(String),
}

pub use domain::{ToolEnvironmentStatus, ToolInstallation};

pub struct Workspace {
    store: Store,
    codex_paths: CodexPaths,
    claude_paths: ClaudePaths,
    grok_paths: GrokPaths,
    opencode_paths: OpenCodePaths,
    pi_paths: PiPaths,
    zcode_paths: ZCodePaths,
    workbuddy_paths: WorkBuddyPaths,
    cursor_gateway: Arc<Mutex<Option<CursorGatewayRuntime>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppSnapshot {
    pub app: AppKind,
    pub providers: Vec<Provider>,
    pub current_id: Option<String>,
}

// Retain CodexSnapshot alias for compatibility if needed
pub type CodexSnapshot = AppSnapshot;

impl Workspace {
    pub fn open(
        db_path: impl AsRef<Path>,
        codex_home: Option<&Path>,
    ) -> Result<Self, SessionError> {
        let store = Store::open(db_path.as_ref())?;
        let settings = store.settings()?;
        let override_codex = codex_home
            .map(Path::to_path_buf)
            .or(settings.codex_home.clone());
        let codex_paths = resolve_codex_paths(override_codex.as_deref())?;
        let claude_paths = resolve_claude_paths(settings.claude_home.as_deref())?;
        let grok_paths = resolve_grok_paths(settings.grok_home.as_deref())?;
        let opencode_paths = resolve_opencode_paths(settings.opencode_home.as_deref())?;
        let pi_paths = resolve_pi_paths(settings.pi_home.as_deref())?;
        let zcode_paths = resolve_zcode_paths(settings.zcode_home.as_deref())?;
        let workbuddy_paths = resolve_workbuddy_paths(settings.workbuddy_home.as_deref())?;
        let cursor_gateway = match CursorGatewayRuntime::new() {
            Ok(gw) => Arc::new(Mutex::new(Some(gw))),
            Err(e) => {
                tracing::warn!(%e, "could not initialize CursorGatewayRuntime");
                Arc::new(Mutex::new(None))
            }
        };
        let mut ws = Self {
            store,
            codex_paths,
            claude_paths,
            grok_paths,
            opencode_paths,
            pi_paths,
            zcode_paths,
            workbuddy_paths,
            cursor_gateway,
        };
        let _ = ws.sync_zcode_from_live();
        let _ = ws.sync_workbuddy_from_live();
        let _ = ws.prune_logs(settings.log_config.retention_days);
        ws.write_diagnostic_log(
            LogLevel::Info,
            "workspace",
            &format!("Workspace initialized (db: {})", db_path.as_ref().display()),
        );
        Ok(ws)
    }

    pub fn sync_zcode_from_live(&self) -> Result<(), SessionError> {
        let custom_providers = list_custom_providers_from_config(&self.zcode_paths)?;
        let now = now_secs();
        for (idx, (id, name, website_url, settings)) in custom_providers.into_iter().enumerate() {
            let existing = self.store.get_provider(&id)?;
            let provider = Provider {
                id,
                app: AppKind::ZCode,
                name,
                website_url,
                settings: ProviderSettings::ZCode(settings),
                created_at: existing.as_ref().map(|p| p.created_at).unwrap_or(now),
                sort_index: existing
                    .as_ref()
                    .map(|p| p.sort_index)
                    .unwrap_or(idx as i64),
            };
            self.store.upsert_provider(&provider)?;
        }
        Ok(())
    }

    pub fn open_default() -> Result<Self, SessionError> {
        Self::open(store::default_db_path()?, None)
    }

    pub fn data_dir(&self) -> &Path {
        self.store.data_dir()
    }

    pub fn codex_home(&self) -> &Path {
        &self.codex_paths.home
    }

    pub fn claude_home(&self) -> &Path {
        &self.claude_paths.home
    }

    pub fn grok_home(&self) -> &Path {
        &self.grok_paths.home
    }

    pub fn opencode_home(&self) -> &Path {
        &self.opencode_paths.home
    }

    pub fn pi_home(&self) -> &Path {
        &self.pi_paths.home
    }

    pub fn zcode_home(&self) -> &Path {
        &self.zcode_paths.home
    }

    pub fn settings(&self) -> Result<AppSettings, SessionError> {
        Ok(self.store.settings()?)
    }

    pub fn save_settings(&self, settings: AppSettings) -> Result<(), SessionError> {
        self.store.save_settings(&settings)?;
        Ok(())
    }

    pub fn log_config(&self) -> Result<LogConfig, SessionError> {
        Ok(self.store.settings()?.log_config)
    }

    pub fn set_log_config(&self, log_config: LogConfig) -> Result<(), SessionError> {
        let mut settings = self.store.settings()?;
        settings.log_config = log_config;
        self.store.save_settings(&settings)?;
        self.write_diagnostic_log(
            LogLevel::Info,
            "settings",
            &format!(
                "Diagnostic log settings updated: enabled={}, level={}, retention_days={}",
                settings.log_config.enabled,
                settings.log_config.level.as_str(),
                settings.log_config.retention_days
            ),
        );
        Ok(())
    }

    pub fn log_dir(&self) -> PathBuf {
        self.store.data_dir().join("logs")
    }

    pub fn main_log_path(&self) -> PathBuf {
        self.log_dir().join("router-switch.log")
    }

    pub fn clear_logs(&self) -> Result<(), SessionError> {
        let log_dir = self.log_dir();
        if log_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&log_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let _ = std::fs::remove_file(&path);
                    }
                }
            }
        }
        self.write_diagnostic_log(LogLevel::Info, "log", "Diagnostic logs cleared");
        Ok(())
    }

    pub fn prune_logs(&self, retention_days: u32) -> Result<(), SessionError> {
        if retention_days == 0 {
            return Ok(());
        }
        let log_dir = self.log_dir();
        if !log_dir.exists() {
            return Ok(());
        }
        let max_age_secs = (retention_days as u64) * 86400;
        let now = std::time::SystemTime::now();

        if let Ok(entries) = std::fs::read_dir(&log_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Ok(meta) = path.metadata() {
                        if let Ok(modified) = meta.modified() {
                            if let Ok(duration) = now.duration_since(modified) {
                                if duration.as_secs() > max_age_secs {
                                    let _ = std::fs::remove_file(&path);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn write_diagnostic_log(&self, level: LogLevel, tag: &str, message: &str) {
        let config = match self.store.settings() {
            Ok(s) => s.log_config,
            Err(_) => return,
        };
        if !config.enabled {
            return;
        }
        if level.priority() > config.level.priority() {
            return;
        }

        let log_dir = self.log_dir();
        let _ = std::fs::create_dir_all(&log_dir);
        let log_path = self.main_log_path();

        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let line = format!(
            "[{}] [{:<5}] [{}] {}\n",
            timestamp,
            level.as_str().to_uppercase(),
            tag,
            message
        );

        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            use std::io::Write;
            let _ = file.write_all(line.as_bytes());
        }
    }

    pub fn apply_codex_home(&mut self, home: Option<PathBuf>) -> Result<(), SessionError> {
        let mut settings = self.store.settings()?;
        settings.codex_home = home.clone();
        self.store.save_settings(&settings)?;
        self.codex_paths = resolve_codex_paths(home.as_deref())?;
        Ok(())
    }

    pub fn apply_claude_home(&mut self, home: Option<PathBuf>) -> Result<(), SessionError> {
        let mut settings = self.store.settings()?;
        settings.claude_home = home.clone();
        self.store.save_settings(&settings)?;
        self.claude_paths = resolve_claude_paths(home.as_deref())?;
        Ok(())
    }

    pub fn apply_grok_home(&mut self, home: Option<PathBuf>) -> Result<(), SessionError> {
        let mut settings = self.store.settings()?;
        settings.grok_home = home.clone();
        self.store.save_settings(&settings)?;
        self.grok_paths = resolve_grok_paths(home.as_deref())?;
        Ok(())
    }

    pub fn apply_opencode_home(&mut self, home: Option<PathBuf>) -> Result<(), SessionError> {
        let mut settings = self.store.settings()?;
        settings.opencode_home = home.clone();
        self.store.save_settings(&settings)?;
        self.opencode_paths = resolve_opencode_paths(home.as_deref())?;
        Ok(())
    }

    pub fn apply_pi_home(&mut self, home: Option<PathBuf>) -> Result<(), SessionError> {
        let mut settings = self.store.settings()?;
        settings.pi_home = home.clone();
        self.store.save_settings(&settings)?;
        self.pi_paths = resolve_pi_paths(home.as_deref())?;
        Ok(())
    }

    pub fn sync_workbuddy_from_live(&mut self) -> Result<(), SessionError> {
        let live_models = match read_workbuddy_live(&self.workbuddy_paths) {
            Ok(m) => m,
            Err(_) => return Ok(()),
        };
        let existing = self.store.list_providers(AppKind::WorkBuddy)?;
        for item in &live_models {
            if let Some(p) = existing.iter().find(|p| {
                if let ProviderSettings::WorkBuddy(ref s) = p.settings {
                    s.model_id == item.id
                } else {
                    false
                }
            }) {
                let (settings, name) = WorkBuddySettings::from_model_item(item);
                let mut updated = p.clone();
                updated.name = name;
                updated.settings = ProviderSettings::WorkBuddy(settings);
                self.store.upsert_provider(&updated)?;
            } else {
                let (settings, name) = WorkBuddySettings::from_model_item(item);
                let provider = Provider {
                    id: domain::new_provider_id(&item.name),
                    app: AppKind::WorkBuddy,
                    name,
                    website_url: None,
                    settings: ProviderSettings::WorkBuddy(settings),
                    created_at: now_secs(),
                    sort_index: 0,
                };
                self.store.upsert_provider(&provider)?;
            }
        }
        Ok(())
    }

    pub fn workbuddy_home(&self) -> &Path {
        &self.workbuddy_paths.home
    }

    pub fn apply_workbuddy_home(&mut self, home: Option<PathBuf>) -> Result<(), SessionError> {
        let mut settings = self.store.settings()?;
        settings.workbuddy_home = home.clone();
        self.store.save_settings(&settings)?;
        self.workbuddy_paths = resolve_workbuddy_paths(home.as_deref())?;
        self.sync_workbuddy_from_live()?;
        Ok(())
    }

    pub fn apply_zcode_home(&mut self, home: Option<PathBuf>) -> Result<(), SessionError> {
        let mut settings = self.store.settings()?;
        settings.zcode_home = home.clone();
        self.store.save_settings(&settings)?;
        self.zcode_paths = resolve_zcode_paths(home.as_deref())?;
        Ok(())
    }

    pub fn set_theme(&self, theme: ThemePreference) -> Result<(), SessionError> {
        let mut settings = self.store.settings()?;
        settings.theme = theme;
        self.store.save_settings(&settings)?;
        Ok(())
    }

    pub fn set_language(&self, language: AppLanguage) -> Result<(), SessionError> {
        let mut settings = self.store.settings()?;
        settings.language = language;
        self.store.save_settings(&settings)?;
        Ok(())
    }

    pub fn toggle_main_app(&self, app_id: &str) -> Result<bool, SessionError> {
        let mut settings = self.store.settings()?;
        let is_enabled = if settings.main_apps.iter().any(|a| a == app_id) {
            settings.main_apps.retain(|a| a != app_id);
            false
        } else {
            settings.main_apps.push(app_id.to_string());
            true
        };
        self.store.save_settings(&settings)?;
        Ok(is_enabled)
    }

    pub fn reorder_main_apps(&self, new_order: Vec<String>) -> Result<(), SessionError> {
        let mut settings = self.store.settings()?;
        settings.main_apps = new_order;
        self.store.save_settings(&settings)?;
        Ok(())
    }

    pub fn set_launch_on_startup(&self, enabled: bool) -> Result<(), SessionError> {
        let mut settings = self.store.settings()?;
        settings.launch_on_startup = enabled;
        self.store.save_settings(&settings)?;
        Ok(())
    }

    pub fn set_minimize_to_tray(&self, enabled: bool) -> Result<(), SessionError> {
        let mut settings = self.store.settings()?;
        settings.minimize_to_tray = enabled;
        self.store.save_settings(&settings)?;
        Ok(())
    }

    pub fn inspect_environment(&self, fetch_remote: bool) -> Vec<ToolEnvironmentStatus> {
        inspect_all_tools(fetch_remote)
    }

    pub fn inspect_tool(
        &self,
        tool_id: &str,
        display_name: &str,
        fetch_remote: bool,
    ) -> ToolEnvironmentStatus {
        inspect_tool_environment(tool_id, display_name, fetch_remote)
    }

    pub fn snapshot(&self) -> Result<AppSnapshot, SessionError> {
        self.snapshot_for(AppKind::Codex)
    }

    pub fn snapshot_for(&self, app: AppKind) -> Result<AppSnapshot, SessionError> {
        Ok(AppSnapshot {
            app,
            providers: self.store.list_providers(app)?,
            current_id: self.store.current_id(app)?,
        })
    }

    pub fn form_for(&self, id: &str) -> Result<ProviderForm, SessionError> {
        let provider = self.require(id)?;
        match &provider.settings {
            ProviderSettings::Codex(settings) => {
                let mut settings = settings.clone();
                if self.store.current_id(AppKind::Codex)?.as_deref() == Some(id) {
                    let live = read_codex_live(&self.codex_paths)?;
                    backfill_codex_settings(&mut settings, &live.auth, &live.config_toml);
                }
                Ok(ProviderForm::Codex(settings.form_snapshot(
                    &provider.name,
                    provider.website_url.as_deref(),
                )))
            }
            ProviderSettings::Claude(settings) => {
                let mut settings = settings.clone();
                if self.store.current_id(AppKind::Claude)?.as_deref() == Some(id) {
                    let live = read_claude_live(&self.claude_paths)?;
                    backfill_claude_settings(&mut settings, &live.settings);
                }
                Ok(ProviderForm::Claude(settings.form_snapshot(
                    &provider.name,
                    provider.website_url.as_deref(),
                )))
            }
            ProviderSettings::Grok(settings) => {
                let mut settings = settings.clone();
                if self.store.current_id(AppKind::Grok)?.as_deref() == Some(id) {
                    let live = read_grok_live(&self.grok_paths)?;
                    backfill_grok_settings(&mut settings, &live.config_toml);
                }
                Ok(ProviderForm::Grok(settings.form_snapshot(
                    &provider.name,
                    provider.website_url.as_deref(),
                )))
            }
            ProviderSettings::OpenCode(settings) => Ok(ProviderForm::OpenCode(
                settings.form_snapshot(&provider.name, provider.website_url.as_deref()),
            )),
            ProviderSettings::Pi(settings) => Ok(ProviderForm::Pi(
                settings.form_snapshot(&provider.name, provider.website_url.as_deref()),
            )),
            ProviderSettings::Cursor(settings) => {
                let mut settings = settings.clone();
                backfill_cursor_settings(&mut settings);
                Ok(ProviderForm::Cursor(settings.form_snapshot(
                    &provider.name,
                    provider.website_url.as_deref(),
                )))
            }
            ProviderSettings::ZCode(settings) => Ok(ProviderForm::ZCode(
                settings.form_snapshot(&provider.name, provider.website_url.as_deref()),
            )),
            ProviderSettings::WorkBuddy(settings) => Ok(ProviderForm::WorkBuddy(
                settings.form_snapshot(&provider.name, provider.website_url.as_deref()),
            )),
            ProviderSettings::Unsupported { app } => Err(SessionError::Message(format!(
                "暂不支持应用 {} 的表单配置",
                app.display_name()
            ))),
        }
    }

    pub fn save_codex_form(
        &self,
        editing_id: Option<&str>,
        form: CodexForm,
    ) -> Result<Provider, SessionError> {
        self.save_form(AppKind::Codex, editing_id, ProviderForm::Codex(form))
    }

    pub fn save_claude_form(
        &self,
        editing_id: Option<&str>,
        form: ClaudeForm,
    ) -> Result<Provider, SessionError> {
        self.save_form(AppKind::Claude, editing_id, ProviderForm::Claude(form))
    }

    pub fn save_grok_form(
        &self,
        editing_id: Option<&str>,
        form: GrokForm,
    ) -> Result<Provider, SessionError> {
        self.save_form(AppKind::Grok, editing_id, ProviderForm::Grok(form))
    }

    pub fn save_opencode_form(
        &self,
        editing_id: Option<&str>,
        form: OpenCodeForm,
    ) -> Result<Provider, SessionError> {
        self.save_form(AppKind::OpenCode, editing_id, ProviderForm::OpenCode(form))
    }

    pub fn save_pi_form(
        &self,
        editing_id: Option<&str>,
        form: PiForm,
    ) -> Result<Provider, SessionError> {
        self.save_form(AppKind::Pi, editing_id, ProviderForm::Pi(form))
    }

    pub fn save_cursor_form(
        &self,
        editing_id: Option<&str>,
        form: CursorForm,
    ) -> Result<Provider, SessionError> {
        self.save_form(AppKind::Cursor, editing_id, ProviderForm::Cursor(form))
    }

    pub fn save_workbuddy_form(
        &self,
        editing_id: Option<&str>,
        form: WorkBuddyForm,
    ) -> Result<Provider, SessionError> {
        self.save_form(
            AppKind::WorkBuddy,
            editing_id,
            ProviderForm::WorkBuddy(form),
        )
    }

    pub fn save_zcode_form(
        &self,
        editing_id: Option<&str>,
        form: ZCodeForm,
    ) -> Result<Provider, SessionError> {
        self.save_form(AppKind::ZCode, editing_id, ProviderForm::ZCode(form))
    }

    pub fn save_form(
        &self,
        app: AppKind,
        editing_id: Option<&str>,
        form: ProviderForm,
    ) -> Result<Provider, SessionError> {
        let (name, website_url, settings, official_id, is_official) = match form {
            ProviderForm::Codex(f) => {
                let name = f.name.trim().to_string();
                let url = optional_url(&f.website_url);
                let is_off = f.kind.is_official();
                let s = parse_codex_form(f)?;
                (
                    name,
                    url,
                    ProviderSettings::Codex(s),
                    OFFICIAL_CODEX_ID,
                    is_off,
                )
            }
            ProviderForm::Claude(f) => {
                let name = f.name.trim().to_string();
                let url = optional_url(&f.website_url);
                let is_off = f.kind.is_official();
                let s = parse_claude_form(f)?;
                (
                    name,
                    url,
                    ProviderSettings::Claude(s),
                    OFFICIAL_CLAUDE_ID,
                    is_off,
                )
            }
            ProviderForm::Grok(f) => {
                let name = f.name.trim().to_string();
                let url = optional_url(&f.website_url);
                let is_off = f.kind.is_official();
                let s = parse_grok_form(f)?;
                (
                    name,
                    url,
                    ProviderSettings::Grok(s),
                    OFFICIAL_GROK_ID,
                    is_off,
                )
            }
            ProviderForm::OpenCode(f) => {
                let name = f.name.trim().to_string();
                let url = optional_url(&f.website_url);
                let is_off = f.kind.is_official();
                let s = parse_opencode_form(f)?;
                (
                    name,
                    url,
                    ProviderSettings::OpenCode(s),
                    OFFICIAL_OPENCODE_ID,
                    is_off,
                )
            }
            ProviderForm::Pi(f) => {
                let name = f.name.trim().to_string();
                let url = optional_url(&f.website_url);
                let is_off = f.kind.is_official();
                let s = parse_pi_form(f)?;
                (name, url, ProviderSettings::Pi(s), OFFICIAL_PI_ID, is_off)
            }
            ProviderForm::Cursor(f) => {
                let name = f.name.trim().to_string();
                let url = optional_url(&f.website_url);
                let is_off = f.kind.is_official();
                let s = parse_cursor_form(f)?;
                (
                    name,
                    url,
                    ProviderSettings::Cursor(s),
                    OFFICIAL_CURSOR_ID,
                    is_off,
                )
            }
            ProviderForm::WorkBuddy(f) => {
                let name = f.name.trim().to_string();
                let url = optional_url(&f.website_url);
                let is_off = f.kind.is_official();
                let s = parse_workbuddy_form(f)?;
                (
                    name,
                    url,
                    ProviderSettings::WorkBuddy(s),
                    OFFICIAL_WORKBUDDY_ID,
                    is_off,
                )
            }
            ProviderForm::ZCode(f) => {
                let name = f.name.trim().to_string();
                let url = optional_url(&f.website_url);
                let is_off = f.kind.is_official();
                let s = parse_zcode_form(f)?;
                (
                    name,
                    url,
                    ProviderSettings::ZCode(s),
                    OFFICIAL_ZCODE_ID,
                    is_off,
                )
            }
        };

        if app == AppKind::WorkBuddy {
            if let ProviderSettings::WorkBuddy(ref wb_s) = settings {
                if wb_s.kind == domain::WorkBuddyKind::ThirdParty {
                    let existing_providers = self.store.list_providers(AppKind::WorkBuddy)?;
                    for existing in existing_providers {
                        if Some(existing.id.as_str()) != editing_id {
                            if let ProviderSettings::WorkBuddy(ref es) = existing.settings {
                                if es
                                    .model_id
                                    .trim()
                                    .eq_ignore_ascii_case(wb_s.model_id.trim())
                                {
                                    return Err(SessionError::Domain(DomainError::Validation(
                                        format!(
                                            "模型 ID '{}' 已存在，WorkBuddy 模型 ID 不能重复",
                                            wb_s.model_id
                                        ),
                                    )));
                                }
                            }
                        }
                    }
                }
            }
        }

        let provider = if let Some(id) = editing_id {
            let mut existing = self.require(id)?;
            existing.name = name;
            existing.website_url = website_url;
            existing.settings = settings;
            existing
        } else if is_official {
            match self.store.get_provider(official_id)? {
                Some(mut existing) => {
                    existing.name = name;
                    existing.website_url = website_url;
                    existing.settings = settings;
                    existing
                }
                None => Provider {
                    id: official_id.to_string(),
                    app,
                    name,
                    website_url,
                    settings,
                    created_at: now_secs(),
                    sort_index: 0,
                },
            }
        } else {
            let sort_index = next_sort(&self.store, app)?;
            let id = if app == AppKind::ZCode || app == AppKind::WorkBuddy {
                uuid::Uuid::new_v4().to_string()
            } else {
                new_provider_id(&name)
            };
            Provider {
                id,
                app,
                name,
                website_url,
                settings,
                created_at: now_secs(),
                sort_index,
            }
        };

        self.store.upsert_provider(&provider)?;
        self.write_diagnostic_log(
            LogLevel::Info,
            "provider",
            &format!("Saved provider: {} ({})", provider.name, provider.id),
        );

        if app == AppKind::WorkBuddy {
            if let ProviderSettings::WorkBuddy(ref s) = provider.settings {
                if s.kind == domain::WorkBuddyKind::ThirdParty {
                    let item = s.to_model_item(&provider.name);
                    let orig_id = editing_id.and_then(|existing_id| {
                        if let Ok(Some(existing_p)) = self.store.get_provider(existing_id) {
                            if let ProviderSettings::WorkBuddy(ref es) = existing_p.settings {
                                return Some(es.model_id.clone());
                            }
                        }
                        None
                    });
                    write_workbuddy_live(&self.workbuddy_paths, orig_id.as_deref(), &item)?;
                }
            }
        }
        if app == AppKind::ZCode {
            if let ProviderSettings::ZCode(ref s) = provider.settings {
                if s.kind == domain::ZCodeKind::ThirdParty {
                    write_zcode_live(&self.zcode_paths, &provider.id, &provider.name, s)?;
                } else if self.store.current_id(app)?.as_deref() == Some(provider.id.as_str()) {
                    self.write_live(&provider)?;
                }
            }
        } else if self.store.current_id(app)?.as_deref() == Some(provider.id.as_str()) {
            self.write_live(&provider)?;
        }
        Ok(provider)
    }

    pub fn import_from_deeplink(&self, url: &str) -> Result<(Provider, bool), SessionError> {
        let request = domain::parse_deeplink_url(url)?;
        let (app, form, is_enabled) = request.to_provider_form()?;
        let provider = self.save_form(app, None, form)?;
        if is_enabled && app != AppKind::ZCode && app != AppKind::WorkBuddy {
            self.enable(&provider.id)?;
        }
        self.write_diagnostic_log(
            LogLevel::Info,
            "deeplink",
            &format!(
                "Imported provider from deeplink: {} (enabled: {})",
                provider.name, is_enabled
            ),
        );
        Ok((provider, is_enabled))
    }

    pub fn enable(&self, id: &str) -> Result<(), SessionError> {
        let provider = self.require(id)?;
        self.write_live(&provider)?;
        self.store.set_current(provider.app, id)?;
        self.write_diagnostic_log(
            LogLevel::Info,
            "provider",
            &format!(
                "Switched active provider for {} to: {} ({})",
                provider.app.display_name(),
                provider.name,
                provider.id
            ),
        );
        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<(), SessionError> {
        let provider = self.store.get_provider(id)?;
        if let Some(ref p) = provider {
            if p.app == AppKind::WorkBuddy {
                if let ProviderSettings::WorkBuddy(ref s) = p.settings {
                    delete_workbuddy_live(&self.workbuddy_paths, &s.model_id)?;
                }
            }
            if p.app == AppKind::ZCode {
                delete_zcode_live(&self.zcode_paths, id)?;
            }
            self.write_diagnostic_log(
                LogLevel::Info,
                "provider",
                &format!("Deleted provider: {} ({})", p.name, id),
            );
        }
        self.store.delete_provider(id)?;
        Ok(())
    }

    pub fn duplicate(&self, id: &str) -> Result<Provider, SessionError> {
        let source = self.require(id)?;
        let mut copy = source.clone();
        if copy.app == AppKind::WorkBuddy {
            copy.id = uuid::Uuid::new_v4().to_string();
            if let ProviderSettings::WorkBuddy(ref mut s) = copy.settings {
                let base_id = s.model_id.clone();
                let existing_providers = self.store.list_providers(AppKind::WorkBuddy)?;
                let mut candidate = format!("{}-copy", base_id);
                let mut counter = 2;
                while existing_providers.iter().any(|p| {
                    if let ProviderSettings::WorkBuddy(ref es) = p.settings {
                        es.model_id == candidate
                    } else {
                        false
                    }
                }) {
                    candidate = format!("{}-copy-{}", base_id, counter);
                    counter += 1;
                }
                s.model_id = candidate;
            }
        } else if copy.app == AppKind::ZCode {
            copy.id = uuid::Uuid::new_v4().to_string();
        } else {
            copy.id = new_provider_id(&format!("{}-copy", source.name));
        }
        copy.name = format!("{} copy", source.name);
        copy.created_at = now_secs();
        copy.sort_index = next_sort(&self.store, source.app)?;
        self.store.upsert_provider(&copy)?;
        if copy.app == AppKind::WorkBuddy {
            if let ProviderSettings::WorkBuddy(ref s) = copy.settings {
                if s.kind == domain::WorkBuddyKind::ThirdParty {
                    let item = s.to_model_item(&copy.name);
                    write_workbuddy_live(&self.workbuddy_paths, None, &item)?;
                }
            }
        }
        if copy.app == AppKind::ZCode {
            if let ProviderSettings::ZCode(ref s) = copy.settings {
                if s.kind == domain::ZCodeKind::ThirdParty {
                    write_zcode_live(&self.zcode_paths, &copy.id, &copy.name, s)?;
                }
            }
        }
        self.write_diagnostic_log(
            LogLevel::Info,
            "provider",
            &format!("Duplicated provider: {} -> {}", source.name, copy.name),
        );
        Ok(copy)
    }

    fn write_live(&self, provider: &Provider) -> Result<(), SessionError> {
        match &provider.settings {
            ProviderSettings::Codex(settings) => {
                write_codex_live(&self.codex_paths, settings)?;
            }
            ProviderSettings::Claude(settings) => {
                write_claude_live(&self.claude_paths, settings)?;
            }
            ProviderSettings::Grok(settings) => {
                write_grok_live(&self.grok_paths, settings)?;
            }
            ProviderSettings::OpenCode(settings) => {
                write_opencode_live(&self.opencode_paths, &provider.id, &provider.name, settings)?;
            }
            ProviderSettings::Pi(settings) => {
                write_pi_live(&self.pi_paths, &provider.id, settings)?;
            }
            ProviderSettings::Cursor(settings) => {
                if let Some(guard) = self.cursor_gateway.try_lock() {
                    if let Some(gw) = guard.as_ref() {
                        gw.set_settings(Some(settings.clone()));
                    }
                }
            }
            ProviderSettings::WorkBuddy(settings) => {
                if settings.kind == domain::WorkBuddyKind::ThirdParty {
                    let item = settings.to_model_item(&provider.name);
                    write_workbuddy_live(&self.workbuddy_paths, None, &item)?;
                }
            }
            ProviderSettings::ZCode(settings) => {
                write_zcode_live(&self.zcode_paths, &provider.id, &provider.name, settings)?;
            }
            ProviderSettings::Unsupported { app } => {
                return Err(SessionError::Message(format!(
                    "暂不支持应用 {} 的切换操作",
                    app.display_name()
                )));
            }
        }
        Ok(())
    }

    pub fn start_cursor_gateway(
        &self,
        proxy_port: Option<u16>,
        backend_port: Option<u16>,
    ) -> Result<(u16, u16), SessionError> {
        let rt = tokio_runtime();
        rt.block_on(async {
            let mut guard = self.cursor_gateway.lock();
            let gw = match guard.as_mut() {
                Some(gw) => gw,
                None => {
                    let new_gw = CursorGatewayRuntime::new().map_err(|e| {
                        SessionError::Message(format!("初始化 Cursor 网关失败: {e}"))
                    })?;
                    *guard = Some(new_gw);
                    guard.as_mut().unwrap()
                }
            };
            // Sync current cursor provider settings
            if let Ok(Some(current_id)) = self.store.current_id(AppKind::Cursor) {
                if let Ok(Some(provider)) = self.store.get_provider(&current_id) {
                    if let ProviderSettings::Cursor(s) = provider.settings {
                        gw.set_settings(Some(s));
                    }
                }
            }
            let ports = gw
                .start(proxy_port, backend_port)
                .await
                .map_err(|e| SessionError::Message(format!("启动 Cursor 网关失败: {e}")))?;
            Ok(ports)
        })
    }

    pub fn stop_cursor_gateway(&self) -> Result<(), SessionError> {
        let rt = tokio_runtime();
        rt.block_on(async {
            let mut guard = self.cursor_gateway.lock();
            if let Some(gw) = guard.as_mut() {
                gw.stop()
                    .await
                    .map_err(|e| SessionError::Message(format!("停止 Cursor 网关失败: {e}")))?;
            }
            Ok(())
        })
    }

    pub fn is_cursor_gateway_running(&self) -> bool {
        let guard = self.cursor_gateway.lock();
        guard.as_ref().map(|gw| gw.is_running()).unwrap_or(false)
    }

    pub fn cursor_proxy_port(&self) -> u16 {
        let guard = self.cursor_gateway.lock();
        guard
            .as_ref()
            .and_then(|gw| gw.proxy_port())
            .unwrap_or(2080)
    }

    pub fn cursor_backend_port(&self) -> u16 {
        let guard = self.cursor_gateway.lock();
        guard
            .as_ref()
            .and_then(|gw| gw.backend_port())
            .unwrap_or(2081)
    }

    pub fn cursor_ca_state(&self) -> bool {
        let guard = self.cursor_gateway.lock();
        guard
            .as_ref()
            .and_then(|gw| gw.ca_state().ok())
            .map(|s| s.is_trusted())
            .unwrap_or(false)
    }

    pub fn cursor_ca_install_command(&self) -> String {
        let guard = self.cursor_gateway.lock();
        guard
            .as_ref()
            .and_then(|gw| gw.ca_install_command())
            .unwrap_or_else(|| "sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain ~/.router-switch/ca/ca.crt".into())
    }

    fn require(&self, id: &str) -> Result<Provider, SessionError> {
        self.store
            .get_provider(id)?
            .ok_or_else(|| SessionError::Message("服务商不存在".into()))
    }
}

pub fn reveal_path_in_explorer(path: &Path) -> std::io::Result<()> {
    if !path.exists() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(path).spawn()?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer").arg(path).spawn()?;
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        std::process::Command::new("xdg-open").arg(path).spawn()?;
    }
    Ok(())
}

fn optional_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn next_sort(store: &Store, app: AppKind) -> Result<i64, StoreError> {
    let list = store.list_providers(app)?;
    let max = list.iter().map(|p| p.sort_index).max().unwrap_or(0);
    Ok(max + 10)
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{ClaudeKind, CodexKind, CursorKind, GrokKind, OpenCodeKind, PiKind, ZCodeKind};
    use tempfile::TempDir;

    #[test]
    fn enable_codex_third_party_writes_live_files() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("app.db");
        let codex_home = temp.path().join(".codex");
        let ws = Workspace::open(&db_path, Some(&codex_home)).unwrap();

        let form = CodexForm {
            name: "PackyCode".into(),
            website_url: "https://www.packyapi.ai".into(),
            kind: CodexKind::ResponsesThirdParty,
            api_key: "sk-live-test".into(),
            base_url: "https://www.packyapi.ai/v1".into(),
            model: "gpt-5.6-sol".into(),
            model_mappings: Vec::new(),
        };
        let provider = ws.save_codex_form(None, form).unwrap();
        ws.enable(&provider.id).unwrap();

        let snapshot = ws.snapshot_for(AppKind::Codex).unwrap();
        assert_eq!(snapshot.current_id.as_deref(), Some(provider.id.as_str()));
    }

    #[test]
    fn claude_provider_flow() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("app.db");
        let mut ws = Workspace::open(&db_path, None).unwrap();
        ws.apply_claude_home(Some(temp.path().join("claude")))
            .unwrap();

        let form = ClaudeForm {
            name: "OpenRouter".into(),
            website_url: "https://openrouter.ai".into(),
            kind: ClaudeKind::ThirdParty,
            api_key: "sk-or-test".into(),
            base_url: "https://openrouter.ai/api".into(),
            model: "anthropic/claude-3.7-sonnet".into(),
            model_mappings: Vec::new(),
        };
        let provider = ws.save_claude_form(None, form).unwrap();
        ws.enable(&provider.id).unwrap();

        let snapshot = ws.snapshot_for(AppKind::Claude).unwrap();
        assert_eq!(snapshot.current_id.as_deref(), Some(provider.id.as_str()));
    }

    #[test]
    fn grok_provider_flow() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("app.db");
        let mut ws = Workspace::open(&db_path, None).unwrap();
        ws.apply_grok_home(Some(temp.path().join("grok"))).unwrap();

        let form = GrokForm {
            name: "Packy Grok".into(),
            website_url: "https://packy.ai".into(),
            kind: GrokKind::ThirdParty,
            api_key: "xai-key".into(),
            base_url: "https://api.packy.ai/v1".into(),
            model: "grok-4.5".into(),
            model_mappings: Vec::new(),
        };
        let provider = ws.save_grok_form(None, form).unwrap();
        ws.enable(&provider.id).unwrap();

        let snapshot = ws.snapshot_for(AppKind::Grok).unwrap();
        assert_eq!(snapshot.current_id.as_deref(), Some(provider.id.as_str()));
    }

    #[test]
    fn opencode_provider_flow() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("app.db");
        let mut ws = Workspace::open(&db_path, None).unwrap();
        ws.apply_opencode_home(Some(temp.path().join("opencode")))
            .unwrap();

        let form = OpenCodeForm {
            name: "DeepSeek OpenCode".into(),
            website_url: "https://deepseek.com".into(),
            kind: OpenCodeKind::ThirdParty,
            npm: "@ai-sdk/openai-compatible".into(),
            api_key: "sk-ds-key".into(),
            base_url: "https://api.deepseek.com/v1".into(),
            model: "deepseek-chat".into(),
            model_mappings: Vec::new(),
        };
        let provider = ws.save_opencode_form(None, form).unwrap();
        ws.enable(&provider.id).unwrap();

        let snapshot = ws.snapshot_for(AppKind::OpenCode).unwrap();
        assert_eq!(snapshot.current_id.as_deref(), Some(provider.id.as_str()));
    }

    #[test]
    fn pi_provider_flow() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("app.db");
        let mut ws = Workspace::open(&db_path, None).unwrap();
        ws.apply_pi_home(Some(temp.path().join("pi"))).unwrap();

        let form = PiForm {
            name: "S2A Pi".into(),
            website_url: "https://s2a.ii.sb".into(),
            kind: PiKind::ThirdParty,
            api_type: "openai-completions".into(),
            api_key: "sk-s2a-key".into(),
            base_url: "https://s2a.ii.sb/v1".into(),
            model: "grok-4.6".into(),
            model_mappings: Vec::new(),
        };
        let provider = ws.save_pi_form(None, form).unwrap();
        ws.enable(&provider.id).unwrap();

        let snapshot = ws.snapshot_for(AppKind::Pi).unwrap();
        assert_eq!(snapshot.current_id.as_deref(), Some(provider.id.as_str()));
    }

    #[test]
    fn workbuddy_provider_flow() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("app.db");
        let mut ws = Workspace::open(&db_path, None).unwrap();
        let workbuddy_home = temp.path().join("workbuddy");
        ws.apply_workbuddy_home(Some(workbuddy_home.clone()))
            .unwrap();

        let form = WorkBuddyForm {
            name: "Mock WorkBuddy Provider".into(),
            website_url: "https://api.example.com".into(),
            kind: domain::WorkBuddyKind::ThirdParty,
            model_id: "mock-wb-model-1".into(),
            vendor: "Custom".into(),
            base_url: "https://api.example.com/v1".into(),
            api_key: "sk-mock-key-12345".into(),
            supports_tool_call: true,
            supports_images: true,
            supports_reasoning: true,
            reasoning_only: false,
            can_disable_reasoning: true,
            use_custom_protocol: false,
            max_input_tokens: Some(262144),
            max_output_tokens: Some(65536),
            reasoning_effort: "high".into(),
            supported_reasoning_efforts: vec!["medium".into(), "high".into()],
        };
        let provider = ws.save_workbuddy_form(None, form).unwrap();

        // 1. Duplicate model ID is rejected
        let dup_form = WorkBuddyForm {
            name: "Duplicate Model".into(),
            website_url: "".into(),
            kind: domain::WorkBuddyKind::ThirdParty,
            model_id: "mock-wb-model-1".into(),
            vendor: "Custom".into(),
            base_url: "https://api.example.com/v1".into(),
            api_key: "sk-mock-key-12345".into(),
            supports_tool_call: true,
            supports_images: true,
            supports_reasoning: false,
            reasoning_only: false,
            can_disable_reasoning: true,
            use_custom_protocol: false,
            max_input_tokens: Some(262144),
            max_output_tokens: Some(65536),
            reasoning_effort: "medium".into(),
            supported_reasoning_efforts: Vec::new(),
        };
        let dup_err = ws.save_workbuddy_form(None, dup_form);
        assert!(dup_err.is_err(), "Duplicate model ID should be rejected");

        // 1. Immediately written to models.json upon save
        let live_paths = adapters_workbuddy::WorkBuddyPaths::from_home(&workbuddy_home);
        let live = adapters_workbuddy::read_live_models(&live_paths).unwrap();
        assert_eq!(live.len(), 1);
        assert_eq!(live[0].id, "mock-wb-model-1");
        assert_eq!(live[0].name, "Mock WorkBuddy Provider");
        assert_eq!(live[0].url, "https://api.example.com/v1");
        assert!(live[0].supports_reasoning);

        // 2. Duplicate immediately writes copy into models.json
        let _copy = ws.duplicate(&provider.id).unwrap();
        let live_after_dup = adapters_workbuddy::read_live_models(&live_paths).unwrap();
        assert_eq!(live_after_dup.len(), 2);
        assert_eq!(live_after_dup[1].id, "mock-wb-model-1-copy");

        // 3. Delete immediately removes from models.json
        ws.delete(&provider.id).unwrap();
        let live_after_del = adapters_workbuddy::read_live_models(&live_paths).unwrap();
        assert_eq!(live_after_del.len(), 1);
        assert_eq!(live_after_del[0].id, "mock-wb-model-1-copy");
    }

    #[test]
    fn zcode_provider_flow() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("app.db");
        let mut ws = Workspace::open(&db_path, None).unwrap();
        let zcode_home = temp.path().join("zcode");
        ws.apply_zcode_home(Some(zcode_home.clone())).unwrap();

        let form = ZCodeForm {
            name: "cchost".into(),
            website_url: "https://cchost.ai".into(),
            kind: ZCodeKind::ThirdParty,
            provider_kind: "anthropic".into(),
            api_key: "sk-zcode-mock-key-12345".into(),
            base_url: "https://cchost.ai".into(),
            model: "gemini-3.7-flash-high".into(),
            modality_text: true,
            modality_image: true,
            model_mappings: Vec::new(),
        };
        let provider = ws.save_zcode_form(None, form).unwrap();

        // 1. Immediately written to config.json upon save
        let live_paths = adapters_zcode::ZCodePaths::from_home(&zcode_home);
        let live = adapters_zcode::read_live(&live_paths).unwrap();
        assert_eq!(live.config["provider"][&provider.id]["name"], "cchost");
        assert_eq!(
            live.config["provider"][&provider.id]["options"]["baseURL"],
            "https://cchost.ai"
        );

        // 2. Duplicate immediately writes copy into config.json
        let copy = ws.duplicate(&provider.id).unwrap();
        let live_after_dup = adapters_zcode::read_live(&live_paths).unwrap();
        assert_eq!(
            live_after_dup.config["provider"][&copy.id]["name"],
            "cchost copy"
        );

        // 3. Delete immediately removes from config.json
        ws.delete(&provider.id).unwrap();
        let live_after_del = adapters_zcode::read_live(&live_paths).unwrap();
        assert!(live_after_del.config["provider"]
            .get(&provider.id)
            .is_none());
        assert!(live_after_del.config["provider"].get(&copy.id).is_some());
    }

    #[test]
    fn cursor_provider_flow() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("app.db");
        let ws = Workspace::open(&db_path, None).unwrap();

        let form = CursorForm {
            name: "Packy Cursor".into(),
            website_url: "https://packy.ai".into(),
            kind: CursorKind::ThirdParty,
            provider_type: "openai-chat".into(),
            api_key: "sk-cursor-test".into(),
            base_url: "https://api.packy.ai/v1".into(),
            model: "gpt-4o".into(),
            model_mappings: Vec::new(),
        };
        let provider = ws.save_cursor_form(None, form).unwrap();
        ws.enable(&provider.id).unwrap();

        let snapshot = ws.snapshot_for(AppKind::Cursor).unwrap();
        assert_eq!(snapshot.current_id.as_deref(), Some(provider.id.as_str()));
    }

    #[test]
    fn diagnostic_log_operations() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("app.db");
        let ws = Workspace::open(&db_path, None).unwrap();

        let default_config = ws.log_config().unwrap();
        assert!(default_config.enabled);
        assert_eq!(default_config.level, LogLevel::Info);
        assert_eq!(default_config.retention_days, 7);

        // Write log entries
        ws.write_diagnostic_log(LogLevel::Info, "test", "info message 1");
        ws.write_diagnostic_log(
            LogLevel::Debug,
            "test",
            "debug message (filtered by default)",
        );
        ws.write_diagnostic_log(LogLevel::Error, "test", "error message 1");

        let log_file = ws.main_log_path();
        assert!(log_file.exists());
        let content = std::fs::read_to_string(&log_file).unwrap();
        assert!(content.contains("info message 1"));
        assert!(!content.contains("debug message (filtered by default)"));
        assert!(content.contains("error message 1"));

        // Enable debug log
        let new_config = LogConfig {
            enabled: true,
            level: LogLevel::Debug,
            retention_days: 14,
        };
        ws.set_log_config(new_config).unwrap();
        ws.write_diagnostic_log(LogLevel::Debug, "test", "debug message 2");
        let content_after = std::fs::read_to_string(&log_file).unwrap();
        assert!(content_after.contains("debug message 2"));

        // Clear logs
        ws.clear_logs().unwrap();
        // clear_logs removes the files, and logs the cleared message
        assert!(ws.main_log_path().exists());
        let content_cleared = std::fs::read_to_string(&ws.main_log_path()).unwrap();
        assert!(content_cleared.contains("Diagnostic logs cleared"));
        assert!(!content_cleared.contains("info message 1"));
    }

    #[test]
    fn reorder_main_apps_persists() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("app.db");
        let ws = Workspace::open(&db_path, None).unwrap();

        let new_order = vec![
            "grok".into(),
            "claude".into(),
            "codex".into(),
            "opencode".into(),
            "pi".into(),
            "cursor".into(),
        ];
        ws.reorder_main_apps(new_order.clone()).unwrap();

        let loaded = ws.settings().unwrap();
        assert_eq!(loaded.main_apps, new_order);
    }

    #[test]
    fn deeplink_import_flow() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("app.db");
        let mut ws = Workspace::open(&db_path, None).unwrap();
        ws.apply_claude_home(Some(temp.path().join("claude")))
            .unwrap();

        let url = "router-switch://v1/import?resource=provider&app=claude&name=DeepLink%20Claude&endpoint=https%3A%2F%2Fapi.anthropic.com%2Fv1&apiKey=sk-ant-test-key&model=claude-3-7-sonnet&enabled=true";
        let (provider, is_enabled) = ws.import_from_deeplink(url).unwrap();
        assert_eq!(provider.name, "DeepLink Claude");
        assert_eq!(provider.app, AppKind::Claude);
        assert!(is_enabled);

        let snapshot = ws.snapshot_for(AppKind::Claude).unwrap();
        assert_eq!(snapshot.current_id.as_deref(), Some(provider.id.as_str()));
        assert!(snapshot.providers.iter().any(|p| p.id == provider.id));
    }
}

//! App-layer glue: SQLite SSOT + live config adapters for Codex, Claude, Grok, OpenCode, Pi, Cursor, and ZCode.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use adapters_claude::{
    read_live as read_claude_live, resolve_claude_paths,
    write_live_for_provider as write_claude_live, ClaudeAdapterError, ClaudePaths,
};
use adapters_claude_desktop::{
    resolve_claude_desktop_paths, write_live as write_claude_desktop_live, ClaudeDesktopError,
    ClaudeDesktopPaths,
};
use adapters_codex::{
    read_live as read_codex_live, resolve_codex_paths,
    strip_unified_session_bucket as strip_codex_unified_bucket,
    write_live_for_provider_with_options as write_codex_live_with_options, CodexAdapterError,
    CodexPaths, CodexWriteOptions,
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
pub use auth_native::NativeAuthStatus;
pub use cursor_gateway::{CaState, LoadedCa};
use cursor_gateway::{
    CompatGateway, CompatTarget, CursorGatewayRuntime, GatewayError, COMPAT_DEFAULT_PORT,
};
use domain::{
    backfill_claude_settings, backfill_codex_settings, backfill_cursor_settings,
    backfill_grok_settings, extract_claude_api_key, extract_claude_base_url, extract_claude_model,
    extract_codex_api_key, has_login_material, inspect_all_tools, inspect_tool_environment,
    new_provider_id, parse_claude_form, parse_codex_form, parse_cursor_form, parse_grok_form,
    parse_opencode_form, parse_pi_form, parse_workbuddy_form, parse_zcode_form, AppKind,
    ClaudeForm, CodexForm, CursorForm, DomainError, GrokForm, OpenCodeForm, PiForm, Provider,
    ProviderForm, ProviderSettings, RequestProtocol, WorkBuddyForm, WorkBuddySettings, ZCodeForm,
    OFFICIAL_CLAUDE_DESKTOP_ID, OFFICIAL_CLAUDE_ID, OFFICIAL_CODEX_ID, OFFICIAL_CURSOR_ID,
    OFFICIAL_GROK_ID, OFFICIAL_OPENCODE_ID, OFFICIAL_PI_ID, OFFICIAL_WORKBUDDY_ID,
    OFFICIAL_ZCODE_ID,
};
pub use oauth::{
    codex_start_device_flow, xai_start_device_flow, AuthTokens, DeviceCodeStart, DevicePollStatus,
    CODEX_PROVIDER, CODEX_VERIFICATION_URL, XAI_PROVIDER,
};
use parking_lot::Mutex;
use serde_json::json;
use std::sync::OnceLock;
pub use store::{
    AppLanguage, AppSettings, LogConfig, LogLevel, Store, StoreError, ThemePreference,
};
use thiserror::Error;
pub use usage_query::UsageQueryError;

mod auth_native;
pub mod codex_history;
pub mod oauth;
pub mod prompts;
pub mod sessions;
pub mod skills;

pub use prompts::PromptService;

/// 会话/Skills 模块使用的应用标识字符串(与 cc-switch 的 provider id 对齐)
pub mod session_apps {
    pub const CLAUDE: &str = "claude";
    pub const CODEX: &str = "codex";
    pub const GROK: &str = "grok";
    pub const OPENCODE: &str = "opencode";
    pub const PI: &str = "pi";
}

static TOKIO_RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

pub fn tokio_runtime() -> &'static tokio::runtime::Runtime {
    TOKIO_RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to initialize tokio runtime for cursor gateway")
    })
}

/// 一次用量查询的完整执行参数(owned, 可送入后台任务)
#[derive(Debug, Clone)]
pub struct PreparedUsageQuery {
    pub script_code: String,
    pub api_key: String,
    pub base_url: String,
    pub timeout_secs: u64,
    pub access_token: Option<String>,
    pub user_id: Option<String>,
    pub template_type: String,
}

impl PreparedUsageQuery {
    /// 执行 HTTP 请求 + JS 提取, 返回原始结果(未落库)
    pub async fn run(self) -> Result<serde_json::Value, UsageQueryError> {
        usage_query::execute_usage_script(
            &self.script_code,
            &self.api_key,
            &self.base_url,
            self.timeout_secs,
            self.access_token.as_deref(),
            self.user_id.as_deref(),
            Some(&self.template_type),
        )
        .await
    }
}

/// 轮询一次设备码状态(不落库, 供后台循环调用)
pub fn oauth_poll_once(
    provider: &'static str,
    device_code: &str,
    user_code: &str,
    token_endpoint: Option<&str>,
) -> Result<oauth::DevicePollStatus, SessionError> {
    let rt = tokio_runtime();
    rt.block_on(async move {
        match provider {
            oauth::CODEX_PROVIDER => oauth::codex_poll_device(device_code, user_code).await,
            oauth::XAI_PROVIDER => {
                let endpoint = token_endpoint
                    .filter(|s| !s.trim().is_empty())
                    .ok_or_else(|| "缺少 token endpoint".to_string())?;
                oauth::xai_poll_device(device_code, endpoint).await
            }
            other => Err(format!("不支持的认证提供方: {other}")),
        }
    })
    .map_err(SessionError::Message)
}

fn codex_model_mappings(mappings: &[domain::CodexModelMapping]) -> Vec<(String, String)> {
    mappings
        .iter()
        .map(|mapping| (mapping.display_name.clone(), mapping.model.clone()))
        .collect()
}

/// Local base URL that preserves the original provider URL's path (usually
/// `/v1`): tools append their own endpoint suffix (`/responses`,
/// `/chat/completions`, `/v1/messages`), so dropping the path would 404.
/// 模型映射模式: 四档角色 -> (claude-* 路由 ID, 实际请求模型)。
/// 空档沿用 Sonnet(或第一个已填档)的模型，确保子-agent 的 Haiku 可用。
fn desktop_route_model_mappings(mappings: &[domain::ClaudeModelMapping]) -> Vec<(String, String)> {
    let fallback = mappings
        .iter()
        .map(|m| m.model.trim().to_string())
        .find(|model| !model.is_empty());
    domain::CLAUDE_DESKTOP_ROUTES
        .iter()
        .enumerate()
        .filter_map(|(idx, (_role, route_id))| {
            let model = mappings
                .get(idx)
                .map(|m| m.model.trim().to_string())
                .filter(|model| !model.is_empty())
                .or_else(|| fallback.clone())?;
            Some((route_id.to_string(), model))
        })
        .collect()
}

fn gateway_base_url(port: u16, original_base_url: &str) -> String {
    let trimmed = original_base_url.trim().trim_end_matches('/');
    let path = match trimmed.find("://") {
        Some(scheme_end) => {
            let rest = &trimmed[scheme_end + 3..];
            match rest.find('/') {
                Some(slash) => &rest[slash..],
                None => "",
            }
        }
        None => "",
    };
    format!("http://127.0.0.1:{port}{path}")
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
    ClaudeDesktopAdapter(#[from] ClaudeDesktopError),
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
    claude_desktop_paths: ClaudeDesktopPaths,
    grok_paths: GrokPaths,
    opencode_paths: OpenCodePaths,
    pi_paths: PiPaths,
    zcode_paths: ZCodePaths,
    workbuddy_paths: WorkBuddyPaths,
    cursor_gateway: Arc<Mutex<Option<CursorGatewayRuntime>>>,
    compat_gateway: Arc<Mutex<CompatGateway>>,
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
        let claude_desktop_paths =
            resolve_claude_desktop_paths(settings.claude_desktop_home.as_deref())?;
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
            claude_desktop_paths,
            grok_paths,
            opencode_paths,
            pi_paths,
            zcode_paths,
            workbuddy_paths,
            cursor_gateway,
            compat_gateway: Arc::new(Mutex::new(CompatGateway::new())),
        };
        let _ = ws.sync_zcode_from_live();
        let _ = ws.sync_workbuddy_from_live();
        let _ = ws.prune_logs(settings.log_config.retention_days);
        // Tools whose live config points at the compat gateway (a previous
        // session routed them) must find it listening again after a restart.
        ws.restore_routing_gateways();
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

    pub fn apply_claude_desktop_home(&mut self, home: Option<PathBuf>) -> Result<(), SessionError> {
        let mut settings = self.store.settings()?;
        settings.claude_desktop_home = home.clone();
        self.store.save_settings(&settings)?;
        self.claude_desktop_paths = resolve_claude_desktop_paths(home.as_deref())?;
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

    /// 非接管切换第三方 Codex 供应商时是否保留官方登录
    pub fn preserve_codex_official_auth_on_switch(&self) -> Result<bool, SessionError> {
        Ok(self
            .store
            .settings()?
            .preserve_codex_official_auth_on_switch)
    }

    pub fn set_preserve_codex_official_auth(&self, enabled: bool) -> Result<(), SessionError> {
        let mut settings = self.store.settings()?;
        settings.preserve_codex_official_auth_on_switch = enabled;
        self.store.save_settings(&settings)?;
        Ok(())
    }

    /// 统一 Codex 会话历史是否开启
    pub fn unify_codex_session_history(&self) -> Result<bool, SessionError> {
        Ok(self.store.settings()?.unify_codex_session_history)
    }

    fn codex_unify_backup_parent(&self) -> PathBuf {
        store::default_data_dir()
            .unwrap_or_else(|_| PathBuf::from(".router-switch"))
            .join("backups")
            .join("codex-official-history-unify-v1")
    }

    fn codex_unify_restore_backup_dir(&self) -> PathBuf {
        store::default_data_dir()
            .unwrap_or_else(|_| PathBuf::from(".router-switch"))
            .join("backups")
            .join("codex-official-history-unify-restore-v1")
    }

    /// 是否存在可还原的官方会话迁移账本
    pub fn has_codex_unify_history_backup(&self) -> bool {
        codex_history::has_unify_backup(&self.codex_unify_backup_parent(), &self.codex_paths.home)
    }

    /// 切换统一 Codex 会话历史开关。开启且勾选迁入时执行官方会话迁移
    /// (迁移前自动备份，账本绑定当前 Codex 目录)；关闭时清理迁移意愿
    /// 与完成标记。还原由 `restore_codex_unified_history` 单独执行。
    pub fn set_unify_codex_session_history(
        &self,
        enabled: bool,
        migrate_existing: bool,
    ) -> Result<codex_history::HistoryOutcome, SessionError> {
        let mut settings = self.store.settings()?;
        settings.unify_codex_session_history = enabled;
        settings.unify_codex_migrate_existing = enabled && migrate_existing;
        if !enabled {
            settings.codex_official_history_unify = None;
        }
        self.store.save_settings(&settings)?;

        if !enabled || !migrate_existing {
            return Ok(codex_history::HistoryOutcome::skipped("not_requested"));
        }
        self.maybe_migrate_codex_official_history(&settings)
    }

    /// 执行官方会话统一迁移: 已有当前目录的完成标记则跳过；live 未路由
    /// 到 custom 桶(注入被拒绝)时跳过并保留迁移意愿，待下次切换后重试。
    fn maybe_migrate_codex_official_history(
        &self,
        settings: &AppSettings,
    ) -> Result<codex_history::HistoryOutcome, SessionError> {
        let codex_dir_key = fs::canonicalize(&self.codex_paths.home)
            .unwrap_or_else(|_| self.codex_paths.home.clone())
            .to_string_lossy()
            .to_string();
        if let Some(marker) = &settings.codex_official_history_unify {
            if marker.codex_config_dir == codex_dir_key {
                return Ok(codex_history::HistoryOutcome::skipped("already_migrated"));
            }
        }

        let live = read_codex_live(&self.codex_paths)?;
        let generation = self
            .codex_unify_backup_parent()
            .join(chrono::Local::now().format("%Y%m%d-%H%M%S%.3f").to_string());
        let outcome = codex_history::migrate_official_history(
            &self.codex_paths.home,
            &live.config_toml,
            &generation,
        )
        .map_err(SessionError::Message)?;

        if outcome.is_skipped() {
            // live_not_unified 等情况: 保留迁移意愿，清理可能建出的空代际
            let _ = fs::remove_dir(&generation);
            return Ok(outcome);
        }

        // 条件写标记: 迁移期间开关被关掉时不写，避免下次开启被标记挡住
        let mut settings = self.store.settings()?;
        if !settings.unify_codex_session_history || !settings.unify_codex_migrate_existing {
            return Ok(codex_history::HistoryOutcome::skipped(
                "toggle_disabled_during_migration",
            ));
        }
        settings.codex_official_history_unify = Some(store::CodexUnifyMigrationMarker {
            completed_at: chrono::Utc::now().to_rfc3339(),
            codex_config_dir: codex_dir_key,
            migrated_jsonl_files: outcome.jsonl_files as u64,
            migrated_state_rows: outcome.state_rows as u64,
        });
        self.store.save_settings(&settings)?;
        Ok(outcome)
    }

    /// 按迁移账本把官方会话还原回 openai 桶。开关重新开启时拒绝还原。
    pub fn restore_codex_unified_history(
        &self,
    ) -> Result<codex_history::HistoryOutcome, SessionError> {
        if self.unify_codex_session_history()? {
            return Ok(codex_history::HistoryOutcome::skipped("unify_toggle_on"));
        }
        let live = read_codex_live(&self.codex_paths)?;
        codex_history::restore_official_history(
            &self.codex_paths.home,
            &live.config_toml,
            &self.codex_unify_backup_parent(),
            &self.codex_unify_restore_backup_dir(),
        )
        .map_err(SessionError::Message)
    }

    pub fn set_auto_check_update(&self, enabled: bool) -> Result<(), SessionError> {
        let mut settings = self.store.settings()?;
        settings.auto_check_update = enabled;
        self.store.save_settings(&settings)?;
        Ok(())
    }

    pub fn set_skipped_update_version(&self, version: Option<String>) -> Result<(), SessionError> {
        let mut settings = self.store.settings()?;
        settings.skipped_update_version = version;
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
                    // 回填前剥掉统一会话注入产物，避免共享路由进入存储配置
                    let config = strip_codex_unified_bucket(&live.config_toml);
                    backfill_codex_settings(&mut settings, &live.auth, &config);
                }
                Ok(ProviderForm::Codex(settings.form_snapshot(
                    &provider.name,
                    provider.website_url.as_deref(),
                )))
            }
            ProviderSettings::Claude(settings) => {
                let mut settings = settings.clone();
                // Claude Desktop 复用 Claude 结构但 live 是 3P 档案，不做
                // ~/.claude 回填
                if provider.app == AppKind::Claude
                    && self.store.current_id(AppKind::Claude)?.as_deref() == Some(id)
                {
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

    pub fn save_claude_desktop_form(
        &self,
        editing_id: Option<&str>,
        form: ClaudeForm,
    ) -> Result<Provider, SessionError> {
        self.save_form(
            AppKind::ClaudeDesktop,
            editing_id,
            ProviderForm::Claude(form),
        )
    }

    /// 把 Claude Code 中已有的第三方供应商导入 Claude Desktop(对齐 cc-switch)。
    /// 复制为新行而非挪动原行(providers.id 全局唯一，禁止跨应用改写)；
    /// 配置适配为 Claude Desktop 直连所需: env 仅保留 base_url 与 auth
    /// token，模型菜单由 model_mappings 承载(无映射时从 ANTHROPIC_MODEL
    /// 推导)。按确定性 id 或名称+端点去重，重复导入幂等。
    pub fn import_claude_desktop_from_claude(&self) -> Result<usize, SessionError> {
        let claude_providers = self.store.list_providers(AppKind::Claude)?;
        let existing = self.store.list_providers(AppKind::ClaudeDesktop)?;
        let desktop_base_url = |provider: &Provider| -> String {
            match &provider.settings {
                ProviderSettings::Claude(settings) => {
                    extract_claude_base_url(&settings.env).unwrap_or_default()
                }
                _ => String::new(),
            }
        };
        let mut imported = 0usize;
        for source in claude_providers {
            if source.id == OFFICIAL_CLAUDE_ID {
                continue;
            }
            let ProviderSettings::Claude(settings) = source.settings.clone() else {
                continue;
            };
            if settings.kind == domain::ClaudeKind::Official
                || !adapters_claude_desktop::is_compatible_direct_settings(&settings)
            {
                continue;
            }
            let new_id = format!("{}-desktop", source.id);
            let already_imported = existing.iter().any(|p| {
                p.id == new_id
                    || (p.name.trim() == source.name.trim()
                        && desktop_base_url(p) == desktop_base_url(&source))
            });
            if already_imported {
                continue;
            }

            let base_url = extract_claude_base_url(&settings.env).unwrap_or_default();
            let api_key = extract_claude_api_key(&settings.env).unwrap_or_default();
            let mut model_mappings = settings.model_mappings.clone();
            if model_mappings.is_empty() {
                if let Some(model) = extract_claude_model(&settings.env) {
                    model_mappings.push(domain::ClaudeModelMapping {
                        display_name: source.name.trim().to_string(),
                        model,
                        context_window: None,
                        reasoning_effort: None,
                    });
                }
            }
            let provider = Provider {
                id: new_id,
                app: AppKind::ClaudeDesktop,
                name: source.name.clone(),
                website_url: source.website_url.clone(),
                settings: ProviderSettings::Claude(domain::ClaudeSettings {
                    kind: domain::ClaudeKind::ThirdParty,
                    env: json!({
                        "ANTHROPIC_BASE_URL": base_url,
                        "ANTHROPIC_AUTH_TOKEN": api_key,
                    }),
                    request_protocol: RequestProtocol::Anthropic.as_str().into(),
                    model_mappings,
                    desktop_mode: None,
                }),
                created_at: now_secs(),
                sort_index: next_sort(&self.store, AppKind::ClaudeDesktop)?,
            };
            self.store.upsert_provider(&provider)?;
            imported += 1;
        }
        self.write_diagnostic_log(
            LogLevel::Info,
            "provider",
            &format!("Imported {imported} providers from Claude Code into Claude Desktop"),
        );
        Ok(imported)
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
                let official_id = if app == AppKind::ClaudeDesktop {
                    OFFICIAL_CLAUDE_DESKTOP_ID
                } else {
                    OFFICIAL_CLAUDE_ID
                };
                (name, url, ProviderSettings::Claude(s), official_id, is_off)
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
        self.backfill_live_into_current(&provider)?;
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

    /// 对齐 cc-switch: 切换前把当前 live 配置回填到将离开的供应商行，
    /// 保证切回时还原到离开时的样子(含用户在 live 文件里的手动改动)。
    /// 仅覆盖有回填语义的独占文件型应用(Codex / Claude / Grok)。
    fn backfill_live_into_current(&self, target: &Provider) -> Result<(), SessionError> {
        let Some(current_id) = self.store.current_id(target.app)? else {
            return Ok(());
        };
        if current_id == target.id {
            return Ok(());
        }
        let mut current = self.require(&current_id)?;
        match (&mut current.settings, target.app) {
            (ProviderSettings::Codex(settings), AppKind::Codex) => {
                let live = read_codex_live(&self.codex_paths)?;
                let config = strip_codex_unified_bucket(&live.config_toml);
                // 认证仅回填该行能安全承载的形态: 官方行接受 ChatGPT 登录，
                // 第三方行只接受 API Key(避免令牌混入第三方凭据)。
                if has_login_material(&live.auth) {
                    let live_is_bare_key = extract_codex_api_key(&live.auth).is_some();
                    if settings.kind == domain::CodexKind::Official || live_is_bare_key {
                        settings.auth = live.auth.clone();
                    }
                }
                if !config.trim().is_empty() {
                    settings.config_toml = config;
                }
            }
            (ProviderSettings::Claude(settings), AppKind::Claude) => {
                let live = read_claude_live(&self.claude_paths)?;
                *settings = backfill_claude_settings(settings, &live.settings);
            }
            (ProviderSettings::Grok(settings), AppKind::Grok) => {
                let live = read_grok_live(&self.grok_paths)?;
                *settings = backfill_grok_settings(settings, &live.config_toml);
            }
            _ => return Ok(()),
        }
        self.store.upsert_provider(&current)?;
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

    /// OAuth 认证状态(读原生配置/本地存储, 不发起网络请求)
    pub fn oauth_status(
        &self,
        provider: &str,
    ) -> Result<crate::auth_native::NativeAuthStatus, SessionError> {
        crate::auth_native::read_status(provider, &self.store, &self.codex_paths)
    }

    /// 启动设备码登录(短网络调用, 内部 block_on)
    pub fn oauth_start_login(
        &self,
        provider: &str,
    ) -> Result<oauth::DeviceCodeStart, SessionError> {
        let rt = tokio_runtime();
        rt.block_on(async move {
            match provider {
                oauth::CODEX_PROVIDER => oauth::codex_start_device_flow().await,
                oauth::XAI_PROVIDER => oauth::xai_start_device_flow().await,
                other => Err(format!("不支持的认证提供方: {other}")),
            }
        })
        .map_err(SessionError::Message)
    }

    /// 授权完成后持久化凭据(Codex 另写原生 auth.json)
    pub fn oauth_complete(
        &self,
        provider: &str,
        tokens: &oauth::AuthTokens,
    ) -> Result<(), SessionError> {
        crate::auth_native::persist_tokens(provider, tokens, &self.store, &self.codex_paths)
    }

    /// 退出登录(清除本地存储的凭据; Codex 原生 auth.json 一并移除)
    pub fn oauth_logout(&self, provider: &str) -> Result<(), SessionError> {
        crate::auth_native::clear_credentials(provider, &self.store, &self.codex_paths)
    }

    /// 各应用的 skills 目录: (app, dir)
    pub fn skills_roots(&self) -> Vec<(String, PathBuf)> {
        vec![
            (
                session_apps::CLAUDE.to_string(),
                self.claude_paths.home.join("skills"),
            ),
            (
                session_apps::CODEX.to_string(),
                self.codex_paths.home.join("skills"),
            ),
            (
                session_apps::GROK.to_string(),
                self.grok_paths.home.join("skills"),
            ),
            (
                session_apps::OPENCODE.to_string(),
                self.opencode_paths.home.join("skills"),
            ),
            (
                session_apps::PI.to_string(),
                self.pi_paths.home.join("skills"),
            ),
        ]
    }

    /// 扫描全部 Skills(按目录名跨应用归并)
    pub fn scan_skills(&self) -> Vec<skills::SkillEntry> {
        skills::scan_skills(&self.skills_roots())
    }

    /// 把 Skill 安装到目标应用(从任一已有副本拷贝)
    pub fn install_skill(&self, source: &Path, target_app: &str) -> Result<(), String> {
        let roots = self.skills_roots();
        let Some((_, target_dir)) = roots.iter().find(|(app, _)| app == target_app) else {
            return Err(format!("不支持的应用: {target_app}"));
        };
        skills::install_skill(source, target_dir)
    }

    /// 移除某应用下的 Skill 副本
    pub fn remove_skill(&self, path: &Path) -> Result<(), String> {
        skills::remove_skill(path)
    }

    /// 获取指定应用的所有提示词
    pub fn get_prompts(&self, app: AppKind) -> Result<Vec<domain::Prompt>, SessionError> {
        prompts::PromptService::get_prompts(&self.store, app)
    }

    /// 保存或更新提示词
    pub fn save_prompt(
        &self,
        app: AppKind,
        id: &str,
        prompt: domain::Prompt,
    ) -> Result<(), SessionError> {
        prompts::PromptService::save_prompt(&self.store, app, id, prompt)
    }

    /// 删除指定提示词
    pub fn delete_prompt(&self, app: AppKind, id: &str) -> Result<(), SessionError> {
        prompts::PromptService::delete_prompt(&self.store, app, id)
    }

    /// 启用指定提示词（带智能回填与自动备份保护）
    pub fn enable_prompt(&self, app: AppKind, id: &str) -> Result<(), SessionError> {
        prompts::PromptService::enable_prompt(&self.store, app, id)
    }

    /// 停用指定提示词
    pub fn disable_prompt(&self, app: AppKind, id: &str) -> Result<(), SessionError> {
        prompts::PromptService::disable_prompt(&self.store, app, id)
    }

    /// 从现有磁盘配置文件抓取内容新建提示词预设
    pub fn import_prompt_from_file(&self, app: AppKind) -> Result<domain::Prompt, SessionError> {
        prompts::PromptService::import_from_file(&self.store, app)
    }

    /// 首次启动时自动导入现有文件（若数据库中为空）
    pub fn import_prompt_from_file_on_first_launch(
        &self,
        app: AppKind,
    ) -> Result<usize, SessionError> {
        prompts::PromptService::import_from_file_on_first_launch(&self.store, app)
    }

    /// 将数据库中的启用项全量投影到 Live 文件
    pub fn sync_prompt_to_live(&self, app: AppKind) -> Result<(), SessionError> {
        prompts::PromptService::sync_to_live(&self.store, app)
    }

    /// 全量同步所有应用的提示词到磁盘
    pub fn sync_all_prompts_to_live(&self) -> Result<(), SessionError> {
        prompts::PromptService::sync_all_to_live(&self.store)
    }

    /// Skills 中心库目录(~/.router-switch/skills)
    pub fn skills_hub_dir(&self) -> PathBuf {
        store::default_data_dir()
            .unwrap_or_else(|_| PathBuf::from(".router-switch"))
            .join("skills")
    }

    /// Skills 备份目录(~/.router-switch/skills-backup)
    pub fn skills_backup_dir(&self) -> PathBuf {
        store::default_data_dir()
            .unwrap_or_else(|_| PathBuf::from(".router-switch"))
            .join("skills-backup")
    }

    /// 会话根目录: (codex_roots, claude_root)
    pub fn session_roots(&self) -> (Vec<PathBuf>, PathBuf) {
        (
            vec![
                self.codex_paths.home.join("sessions"),
                self.codex_paths.home.join("archived_sessions"),
            ],
            self.claude_paths.home.join("projects"),
        )
    }

    /// 扫描全部会话(最近活跃优先)
    pub fn scan_sessions(&self) -> Vec<sessions::SessionMeta> {
        let (codex_roots, claude_root) = self.session_roots();
        sessions::scan_sessions(&codex_roots, &claude_root)
    }

    /// 加载会话消息预览
    pub fn load_session_messages(
        &self,
        provider_id: &str,
        source_path: &str,
    ) -> Result<Vec<sessions::SessionMessage>, String> {
        let (codex_roots, claude_root) = self.session_roots();
        sessions::load_messages(provider_id, source_path, &codex_roots, &claude_root)
    }

    /// 批量删除会话
    pub fn delete_sessions(
        &self,
        requests: &[sessions::DeleteSessionRequest],
    ) -> Vec<sessions::DeleteSessionOutcome> {
        let (codex_roots, claude_root) = self.session_roots();
        sessions::delete_sessions(requests, &codex_roots, &claude_root)
    }

    /// 保存用量查询配置
    pub fn save_usage_script(
        &self,
        provider_id: &str,
        config: &domain::UsageScriptConfig,
    ) -> Result<(), SessionError> {
        if self.store.get_provider(provider_id)?.is_none() {
            return Err(SessionError::Message("服务商不存在".into()));
        }
        self.store.save_usage_script(provider_id, config)?;
        Ok(())
    }

    pub fn usage_script(
        &self,
        provider_id: &str,
    ) -> Result<Option<domain::UsageScriptConfig>, SessionError> {
        Ok(self.store.usage_script(provider_id)?)
    }

    pub fn delete_usage_script(&self, provider_id: &str) -> Result<(), SessionError> {
        self.store.delete_usage_script(provider_id)?;
        Ok(())
    }

    /// 最近一次用量查询结果与时间戳
    pub fn usage_result(
        &self,
        provider_id: &str,
    ) -> Result<Option<(domain::UsageQueryResult, i64)>, SessionError> {
        Ok(self.store.usage_result(provider_id)?)
    }

    /// 自动刷新调度用的全量清单
    pub fn list_usage_scripts(
        &self,
    ) -> Result<Vec<(String, domain::UsageScriptConfig, Option<i64>)>, SessionError> {
        Ok(self.store.list_usage_scripts()?)
    }

    /// 按 id 获取服务商
    pub fn provider(&self, provider_id: &str) -> Result<Option<Provider>, SessionError> {
        Ok(self.store.get_provider(provider_id)?)
    }

    /// 收集执行用量查询所需的全部参数(同步, 不阻塞); 返回的任务在后台 await。
    pub fn prepare_usage_query(
        &self,
        provider_id: &str,
    ) -> Result<PreparedUsageQuery, SessionError> {
        let Some(config) = self.store.usage_script(provider_id)? else {
            return Err(SessionError::Message("未配置用量查询".into()));
        };
        self.prepare_usage_query_with_config(provider_id, &config)
    }

    /// 同上, 但使用调用方给定的配置(测试脚本按钮: 不必先保存)
    pub fn prepare_usage_query_with_config(
        &self,
        provider_id: &str,
        config: &domain::UsageScriptConfig,
    ) -> Result<PreparedUsageQuery, SessionError> {
        let Some(provider) = self.store.get_provider(provider_id)? else {
            return Err(SessionError::Message("服务商不存在".into()));
        };
        let (provider_base, provider_key) = domain::extract_provider_probe_target(&provider)
            .unwrap_or_else(|_| (String::new(), None));
        let api_key = config
            .effective_api_key(provider_key.as_deref().unwrap_or(""))
            .to_string();
        let base_url = config.effective_base_url(&provider_base).to_string();
        Ok(PreparedUsageQuery {
            script_code: config.code.clone(),
            api_key,
            base_url,
            timeout_secs: config.timeout_secs,
            access_token: config.access_token.clone(),
            user_id: config.user_id.clone(),
            template_type: config.template_type.clone(),
        })
    }

    /// 解析执行结果并落库(同步, 在 await 完成后于 UI 线程调用)。
    pub fn complete_usage_query(
        &self,
        provider_id: &str,
        outcome: Result<serde_json::Value, UsageQueryError>,
    ) -> domain::UsageQueryResult {
        let result = match outcome {
            Ok(value) => domain::parse_usage_result(value),
            Err(err) => domain::failed_usage_result(err.to_string()),
        };
        let _ = self
            .store
            .save_usage_result(provider_id, &result, now_secs());
        result
    }

    /// 同步执行一次用量查询(测试/后台刷新共用; 会阻塞调用线程直到 HTTP 超时)。
    pub fn query_provider_usage_blocking(
        &self,
        provider_id: &str,
    ) -> Result<domain::UsageQueryResult, SessionError> {
        let prepared = self.prepare_usage_query(provider_id)?;
        let rt = tokio_runtime();
        let outcome = rt.block_on(prepared.run());
        Ok(self.complete_usage_query(provider_id, outcome))
    }

    fn write_live(&self, provider: &Provider) -> Result<(), SessionError> {
        // Claude Desktop 复用 Claude 表单结构，但落盘目标是 3P 网关配置；
        // 模型映射模式经本地 compat 网关把 claude-* 角色路由映射到实际模型。
        if provider.app == AppKind::ClaudeDesktop {
            if let ProviderSettings::Claude(settings) = &provider.settings {
                let mut settings = settings.clone();
                let is_mapping =
                    settings.desktop_mode.as_deref() == Some(domain::CLAUDE_DESKTOP_MODE_MAPPING);
                if is_mapping {
                    let base_url = extract_claude_base_url(&settings.env).unwrap_or_default();
                    let api_key = extract_claude_api_key(&settings.env).unwrap_or_default();
                    let model_mappings = desktop_route_model_mappings(&settings.model_mappings);
                    let protocol = RequestProtocol::parse(&settings.request_protocol);
                    let port = self.register_compat_target(CompatTarget {
                        app: AppKind::ClaudeDesktop,
                        model_mappings,
                        base_url: base_url.clone(),
                        api_key,
                        model: settings
                            .model_mappings
                            .first()
                            .map(|m| m.model.clone())
                            .unwrap_or_default(),
                        protocol,
                    })?;
                    if !base_url.is_empty() {
                        settings.env["ANTHROPIC_BASE_URL"] =
                            json!(gateway_base_url(port, &base_url));
                    }
                } else {
                    self.unregister_compat_target(AppKind::ClaudeDesktop);
                }
                return write_claude_desktop_live(&self.claude_desktop_paths, &settings)
                    .map_err(SessionError::from);
            }
        }
        match &provider.settings {
            ProviderSettings::Codex(settings) => {
                let mut settings = settings.clone();
                let app_settings = self.store.settings()?;
                let options = CodexWriteOptions {
                    preserve_official_auth: app_settings.preserve_codex_official_auth_on_switch,
                    unify_session_history: app_settings.unify_codex_session_history,
                };
                match self.routing_plan(AppKind::Codex, provider) {
                    Some(upstream) => {
                        let base_url = domain::extract_codex_base_url(&settings.config_toml)
                            .unwrap_or_default();
                        let port = self.register_compat_target(CompatTarget {
                            app: AppKind::Codex,
                            model_mappings: codex_model_mappings(&settings.model_mappings),
                            base_url: base_url.clone(),
                            api_key: domain::extract_codex_api_key(&settings.auth)
                                .unwrap_or_default(),
                            model: domain::extract_codex_model(&settings.config_toml)
                                .unwrap_or_default(),
                            protocol: upstream,
                        })?;
                        if !base_url.is_empty() {
                            settings.config_toml = settings
                                .config_toml
                                .replace(&base_url, &gateway_base_url(port, &base_url));
                        }
                    }
                    None => self.unregister_compat_target(AppKind::Codex),
                }
                write_codex_live_with_options(&self.codex_paths, &settings, options)?;
            }
            ProviderSettings::Claude(settings) => {
                let mut settings = settings.clone();
                match self.routing_plan(AppKind::Claude, provider) {
                    Some(upstream) => {
                        let port = self.register_compat_target(CompatTarget {
                            app: AppKind::Claude,
                            base_url: domain::extract_claude_base_url(&settings.env)
                                .unwrap_or_default(),
                            api_key: domain::extract_claude_api_key(&settings.env)
                                .unwrap_or_default(),
                            model: domain::extract_claude_model(&settings.env).unwrap_or_default(),
                            protocol: upstream,
                            model_mappings: settings
                                .model_mappings
                                .iter()
                                .map(|m| (m.display_name.clone(), m.model.clone()))
                                .collect(),
                        })?;
                        let claude_base =
                            domain::extract_claude_base_url(&settings.env).unwrap_or_default();
                        if let Some(env) = settings.env.as_object_mut() {
                            env.insert(
                                "ANTHROPIC_BASE_URL".into(),
                                json!(gateway_base_url(port, &claude_base)),
                            );
                        }
                    }
                    None => self.unregister_compat_target(AppKind::Claude),
                }
                write_claude_live(&self.claude_paths, &settings)?;
            }
            ProviderSettings::Grok(settings) => {
                let mut settings = settings.clone();
                match self.routing_plan(AppKind::Grok, provider) {
                    Some(upstream) => {
                        let base_url = domain::extract_grok_base_url(&settings.config_toml)
                            .unwrap_or_default();
                        let port = self.register_compat_target(CompatTarget {
                            app: AppKind::Grok,
                            base_url: base_url.clone(),
                            api_key: domain::extract_grok_api_key(&settings.config_toml)
                                .unwrap_or_default(),
                            model: domain::extract_grok_model(&settings.config_toml)
                                .unwrap_or_default(),
                            protocol: upstream,
                            model_mappings: settings
                                .model_mappings
                                .iter()
                                .map(|m| (m.display_name.clone(), m.model.clone()))
                                .collect(),
                        })?;
                        if !base_url.is_empty() {
                            settings.config_toml = settings
                                .config_toml
                                .replace(&base_url, &gateway_base_url(port, &base_url));
                        }
                    }
                    None => self.unregister_compat_target(AppKind::Grok),
                }
                write_grok_live(&self.grok_paths, &settings)?;
            }
            ProviderSettings::OpenCode(settings) => {
                let mut settings = settings.clone();
                match self.routing_plan(AppKind::OpenCode, provider) {
                    Some(upstream) => {
                        let base_url = domain::extract_opencode_base_url(&settings.options)
                            .unwrap_or_default();
                        let port = self.register_compat_target(CompatTarget {
                            app: AppKind::OpenCode,
                            base_url: base_url.clone(),
                            api_key: domain::extract_opencode_api_key(&settings.options)
                                .unwrap_or_default(),
                            model: domain::extract_opencode_model(&settings.models)
                                .unwrap_or_default(),
                            protocol: upstream,
                            model_mappings: settings
                                .model_mappings
                                .iter()
                                .map(|m| (m.display_name.clone(), m.model_id.clone()))
                                .collect(),
                        })?;
                        if let Some(options) = settings.options.as_object_mut() {
                            options
                                .insert("baseURL".into(), json!(gateway_base_url(port, &base_url)));
                        }
                    }
                    None => self.unregister_compat_target(AppKind::OpenCode),
                }
                write_opencode_live(
                    &self.opencode_paths,
                    &provider.id,
                    &provider.name,
                    &settings,
                )?;
            }
            ProviderSettings::Pi(settings) => {
                let mut settings = settings.clone();
                match self.routing_plan(AppKind::Pi, provider) {
                    Some(upstream) => {
                        let port = self.register_compat_target(CompatTarget {
                            app: AppKind::Pi,
                            base_url: settings.base_url.clone(),
                            api_key: settings.api_key.clone(),
                            model: settings.model.clone(),
                            protocol: upstream,
                            model_mappings: settings
                                .model_mappings
                                .iter()
                                .map(|m| (m.display_name.clone(), m.model_id.clone()))
                                .collect(),
                        })?;
                        settings.base_url = gateway_base_url(port, &settings.base_url);
                    }
                    None => self.unregister_compat_target(AppKind::Pi),
                }
                write_pi_live(&self.pi_paths, &provider.id, &settings)?;
            }
            // ZCode natively switches protocols via its provider_kind, and
            // WorkBuddy has no protocol choice — neither needs the gateway.
            ProviderSettings::ZCode(settings) => {
                write_zcode_live(&self.zcode_paths, &provider.id, &provider.name, settings)?;
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
            ProviderSettings::Unsupported { app } => {
                return Err(SessionError::Message(format!(
                    "暂不支持应用 {} 的切换操作",
                    app.display_name()
                )));
            }
        }
        Ok(())
    }

    /// Apps whose live configs can be routed through the compat gateway.
    const ROUTABLE_APPS: [AppKind; 6] = [
        AppKind::Claude,
        AppKind::ClaudeDesktop,
        AppKind::Codex,
        AppKind::Grok,
        AppKind::OpenCode,
        AppKind::Pi,
    ];

    /// Some(upstream protocol) when the provider's configured request protocol
    /// differs from the tool's native dialect — i.e. traffic must be converted
    /// by the compat gateway instead of written through as a direct connection.
    fn routing_plan(&self, app: AppKind, provider: &Provider) -> Option<RequestProtocol> {
        let protocol_raw = match &provider.settings {
            ProviderSettings::Claude(settings) => &settings.request_protocol,
            ProviderSettings::Codex(settings) => &settings.request_protocol,
            ProviderSettings::Grok(settings) => &settings.request_protocol,
            ProviderSettings::OpenCode(settings) => &settings.request_protocol,
            ProviderSettings::Pi(settings) => &settings.request_protocol,
            _ => return None,
        };
        let configured = RequestProtocol::parse(protocol_raw);
        let native = RequestProtocol::default_for_app(app);
        (configured != native).then_some(configured)
    }

    fn register_compat_target(&self, target: CompatTarget) -> Result<u16, SessionError> {
        self.compat_gateway.lock().registry().set_target(target);
        self.ensure_compat_gateway()
    }

    fn unregister_compat_target(&self, app: AppKind) {
        self.compat_gateway.lock().registry().remove_target(app);
    }

    /// Start the compat listener if it is not running yet, returning its port.
    pub fn ensure_compat_gateway(&self) -> Result<u16, SessionError> {
        let rt = tokio_runtime();
        rt.block_on(async {
            let mut guard = self.compat_gateway.lock();
            if !guard.is_running() {
                guard.start(COMPAT_DEFAULT_PORT).await?;
            }
            Ok(guard.port().unwrap_or(COMPAT_DEFAULT_PORT))
        })
    }

    pub fn compat_gateway_port(&self) -> Option<u16> {
        self.compat_gateway.lock().port()
    }

    pub fn compat_target_for_app(&self, app: AppKind) -> Option<CompatTarget> {
        self.compat_gateway.lock().registry().target_for_app(app)
    }

    pub fn compat_routed_apps(&self) -> Vec<AppKind> {
        self.compat_gateway.lock().registry().routed_apps()
    }

    /// Re-apply the live config of every currently-selected provider that
    /// needs routing, so the gateway is listening again after an app restart
    /// even though nothing was switched in this run.
    fn restore_routing_gateways(&self) {
        for app in Self::ROUTABLE_APPS {
            let Ok(Some(current_id)) = self.store.current_id(app) else {
                continue;
            };
            let Ok(Some(provider)) = self.store.get_provider(&current_id) else {
                continue;
            };
            if self.routing_plan(app, &provider).is_some() {
                if let Err(err) = self.write_live(&provider) {
                    tracing::warn!(%err, app = ?app, "could not restore routed live config");
                }
            }
        }
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
            request_protocol: String::new(),
            model_mappings: Vec::new(),
        };
        let provider = ws.save_codex_form(None, form).unwrap();
        ws.enable(&provider.id).unwrap();

        let snapshot = ws.snapshot_for(AppKind::Codex).unwrap();
        assert_eq!(snapshot.current_id.as_deref(), Some(provider.id.as_str()));
    }

    #[test]
    fn claude_desktop_import_and_direct_write_flow() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("app.db");
        let mut ws = Workspace::open(&db_path, None).unwrap();
        ws.apply_claude_desktop_home(Some(temp.path().join("AppSupport")))
            .unwrap();

        let form = ClaudeForm {
            name: "PackyCode".into(),
            website_url: "https://www.packyapi.ai".into(),
            kind: ClaudeKind::ThirdParty,
            api_key: "sk-mock-key-12345".into(),
            base_url: "https://www.packyapi.ai".into(),
            model: "claude-sonnet-5".into(),
            request_protocol: String::new(),
            model_mappings: Vec::new(),
            desktop_mode: None,
        };
        let claude_provider = ws.save_claude_form(None, form).unwrap();

        // 从 Claude Code 导入 → 复制为新行(id 加 -desktop 后缀)，原行不挪动
        let imported = ws.import_claude_desktop_from_claude().unwrap();
        assert_eq!(imported, 1);
        let desktop = ws.snapshot_for(AppKind::ClaudeDesktop).unwrap();
        let new_id = format!("{}-desktop", claude_provider.id);
        let copied = desktop
            .providers
            .iter()
            .find(|p| p.id == new_id)
            .expect("imported copy should exist in Claude Desktop");
        assert!(copied.id != claude_provider.id);

        // Claude Code 供应商列表不受影响
        let claude_after = ws.snapshot_for(AppKind::Claude).unwrap();
        assert!(claude_after
            .providers
            .iter()
            .any(|p| p.id == claude_provider.id));

        // 配置已适配 Claude Desktop 直连: env 仅保留 base_url + auth token，
        // 模型菜单由 model_mappings 承载(从 ANTHROPIC_MODEL 推导)
        if let ProviderSettings::Claude(settings) = &copied.settings {
            assert_eq!(
                settings.env.get("ANTHROPIC_BASE_URL"),
                Some(&serde_json::json!("https://www.packyapi.ai"))
            );
            assert_eq!(
                settings.env.get("ANTHROPIC_AUTH_TOKEN"),
                Some(&serde_json::json!("sk-mock-key-12345"))
            );
            assert!(settings.env.get("ANTHROPIC_MODEL").is_none());
            assert_eq!(settings.model_mappings.len(), 1);
            assert_eq!(settings.model_mappings[0].model, "claude-sonnet-5");
        } else {
            panic!("expected claude settings for desktop copy");
        }

        // 官方种子已就位
        assert!(desktop
            .providers
            .iter()
            .any(|p| p.id == OFFICIAL_CLAUDE_DESKTOP_ID));

        // 启用复制品 → 写入 3P 网关配置
        ws.enable(&new_id).unwrap();
        let paths = adapters_claude_desktop::ClaudeDesktopPaths::from_app_support(
            temp.path().join("AppSupport"),
        );
        let profile = fs::read_to_string(&paths.profile_path).unwrap();
        assert!(profile.contains("inferenceGatewayBaseUrl"));
        assert!(profile.contains("https://www.packyapi.ai"));
        let normal = fs::read_to_string(&paths.normal_config).unwrap();
        assert!(normal.contains("\"deploymentMode\": \"3p\""));

        // 切回官方 → 1P 模式并移除档案
        ws.enable(OFFICIAL_CLAUDE_DESKTOP_ID).unwrap();
        let normal = fs::read_to_string(&paths.normal_config).unwrap();
        assert!(normal.contains("\"deploymentMode\": \"1p\""));
        assert!(!paths.profile_path.exists());

        // 重复导入幂等
        assert_eq!(ws.import_claude_desktop_from_claude().unwrap(), 0);
    }

    #[test]
    fn claude_desktop_mapping_mode_routes_via_local_gateway() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("app.db");
        let mut ws = Workspace::open(&db_path, None).unwrap();
        ws.apply_claude_desktop_home(Some(temp.path().join("AppSupport")))
            .unwrap();

        let form = ClaudeForm {
            name: "DeepSeek Relay".into(),
            website_url: "https://api.example.com".into(),
            kind: ClaudeKind::ThirdParty,
            api_key: "sk-mock-key-12345".into(),
            base_url: "https://api.example.com".into(),
            model: "deepseek-v4-pro".into(),
            request_protocol: String::new(),
            model_mappings: vec![
                domain::ClaudeModelMapping {
                    display_name: "gemini-3.7-flash-high".into(),
                    model: "gpt-5.4-mini".into(),
                    context_window: Some(domain::CLAUDE_DESKTOP_ONE_M_WINDOW),
                    reasoning_effort: None,
                },
                domain::ClaudeModelMapping {
                    display_name: "DeepSeek V4 Pro".into(),
                    model: "deepseek-v4-pro".into(),
                    context_window: None,
                    reasoning_effort: None,
                },
                domain::ClaudeModelMapping {
                    display_name: String::new(),
                    model: String::new(),
                    context_window: None,
                    reasoning_effort: None,
                },
                domain::ClaudeModelMapping {
                    display_name: "DeepSeek V4 Flash".into(),
                    model: "deepseek-v4-flash".into(),
                    context_window: None,
                    reasoning_effort: None,
                },
            ],
            desktop_mode: Some(domain::CLAUDE_DESKTOP_MODE_MAPPING.into()),
        };
        let provider = ws.save_claude_desktop_form(None, form).unwrap();
        ws.enable(&provider.id).unwrap();

        // 档案暴露 claude-* 安全路由(非实际模型)，端点指向本地网关
        let paths = adapters_claude_desktop::ClaudeDesktopPaths::from_app_support(
            temp.path().join("AppSupport"),
        );
        let profile = fs::read_to_string(&paths.profile_path).unwrap();
        assert!(profile.contains("claude-sonnet-5"));
        assert!(profile.contains("claude-opus-5"));
        assert!(profile.contains("claude-fable-5"));
        assert!(profile.contains("claude-haiku-4-5"));
        assert!(!profile.contains("deepseek-v4-pro"));
        assert!(profile.contains("supports1m"));
        assert!(profile.contains("http://127.0.0.1:"));
        assert!(!profile.contains("https://api.example.com"));

        // 切回直连 → 档案恢复供应商端点
        let mut direct_form = ws.form_for(&provider.id).unwrap();
        if let ProviderForm::Claude(ref mut f) = direct_form {
            f.desktop_mode = Some(domain::CLAUDE_DESKTOP_MODE_DIRECT.into());
        }
        let updated = ws
            .save_claude_desktop_form(
                Some(&provider.id),
                match direct_form {
                    ProviderForm::Claude(f) => f,
                    _ => unreachable!(),
                },
            )
            .unwrap();
        ws.enable(&updated.id).unwrap();
        let profile = fs::read_to_string(&paths.profile_path).unwrap();
        assert!(profile.contains("https://api.example.com"));
        assert!(!profile.contains("http://127.0.0.1:"));
    }

    #[test]
    fn switching_back_to_official_restores_official_login_and_config() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("app.db");
        let codex_home = temp.path().join(".codex");
        fs::create_dir_all(&codex_home).unwrap();
        // 模拟用户官方登录后的 live 状态(ChatGPT OAuth + 自定义官方配置)
        fs::write(
            codex_home.join("auth.json"),
            r#"{"tokens":{"access_token":"chatgpt-oauth","refresh_token":"rt"}}"#,
        )
        .unwrap();
        let official_config = "default_model = \"gpt-5.6\"\ndisable_response_storage = false\n";
        fs::write(codex_home.join("config.toml"), official_config).unwrap();

        let ws = Workspace::open(&db_path, Some(&codex_home)).unwrap();
        let form = CodexForm {
            name: "PackyCode".into(),
            website_url: "https://www.packyapi.ai".into(),
            kind: CodexKind::ResponsesThirdParty,
            api_key: "sk-live-test".into(),
            base_url: "https://www.packyapi.ai/v1".into(),
            model: "gpt-5.6-sol".into(),
            request_protocol: String::new(),
            model_mappings: Vec::new(),
        };
        let third_party_id = ws.save_codex_form(None, form).unwrap();

        // 官方 → 第三方 → 官方: 应回到官方登录与官方配置
        ws.enable(&third_party_id.id).unwrap();
        let tp_auth: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(codex_home.join("auth.json")).unwrap())
                .unwrap();
        assert_eq!(tp_auth["OPENAI_API_KEY"], "sk-live-test");

        ws.enable(OFFICIAL_CODEX_ID).unwrap();
        let back_auth: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(codex_home.join("auth.json")).unwrap())
                .unwrap();
        assert_eq!(back_auth["tokens"]["access_token"], "chatgpt-oauth");
        assert_eq!(
            fs::read_to_string(codex_home.join("config.toml")).unwrap(),
            official_config
        );
    }

    #[test]
    fn switching_back_to_official_survives_unified_route_and_manual_edits() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("app.db");
        let codex_home = temp.path().join(".codex");
        fs::create_dir_all(&codex_home).unwrap();
        fs::write(
            codex_home.join("auth.json"),
            r#"{"tokens":{"access_token":"chatgpt-oauth","refresh_token":"rt"}}"#,
        )
        .unwrap();
        let official_config = "default_model = \"gpt-5.6\"\n";
        fs::write(codex_home.join("config.toml"), official_config).unwrap();

        let ws = Workspace::open(&db_path, Some(&codex_home)).unwrap();
        // 统一会话历史开启: 官方 live 将带共享 custom 路由
        ws.set_unify_codex_session_history(true, false).unwrap();

        let form = CodexForm {
            name: "PackyCode".into(),
            website_url: "https://www.packyapi.ai".into(),
            kind: CodexKind::ResponsesThirdParty,
            api_key: "sk-live-test".into(),
            base_url: "https://www.packyapi.ai/v1".into(),
            model: "gpt-5.6-sol".into(),
            request_protocol: String::new(),
            model_mappings: Vec::new(),
        };
        let third_party = ws.save_codex_form(None, form).unwrap();

        // 官方(注入统一路由) → 第三方 → 官方: 官方登录与配置都不能丢
        ws.enable(OFFICIAL_CODEX_ID).unwrap();
        assert!(fs::read_to_string(codex_home.join("config.toml"))
            .unwrap()
            .contains("model_provider = \"custom\""));
        ws.enable(&third_party.id).unwrap();
        ws.enable(OFFICIAL_CODEX_ID).unwrap();

        let back_auth: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(codex_home.join("auth.json")).unwrap())
                .unwrap();
        assert_eq!(back_auth["tokens"]["access_token"], "chatgpt-oauth");
        let back_config = fs::read_to_string(codex_home.join("config.toml")).unwrap();
        // 用户官方配置保留，且统一路由重新注入
        assert!(back_config.contains("default_model = \"gpt-5.6\""));
        assert!(back_config.contains("model_provider = \"custom\""));
    }

    #[test]
    fn preserve_official_auth_and_unify_history_flags_flow() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("app.db");
        let codex_home = temp.path().join(".codex");
        fs::create_dir_all(&codex_home).unwrap();
        // 预置官方 ChatGPT 登录与官方配置
        fs::write(
            codex_home.join("auth.json"),
            r#"{"tokens":{"access_token":"chatgpt-oauth"}}"#,
        )
        .unwrap();
        fs::write(
            codex_home.join("config.toml"),
            "default_model = \"gpt-5.6\"\n",
        )
        .unwrap();

        let ws = Workspace::open(&db_path, Some(&codex_home)).unwrap();

        // 默认关闭
        assert!(!ws.preserve_codex_official_auth_on_switch().unwrap());
        assert!(!ws.unify_codex_session_history().unwrap());

        // 第三方供应商
        let form = CodexForm {
            name: "PackyCode".into(),
            website_url: "https://www.packyapi.ai".into(),
            kind: CodexKind::ResponsesThirdParty,
            api_key: "sk-live-test".into(),
            base_url: "https://www.packyapi.ai/v1".into(),
            model: "gpt-5.6-sol".into(),
            request_protocol: String::new(),
            model_mappings: Vec::new(),
        };
        let provider = ws.save_codex_form(None, form).unwrap();

        // 开启保留官方登录后切换: auth.json 保留 ChatGPT 登录，密钥走 bearer token
        ws.set_preserve_codex_official_auth(true).unwrap();
        ws.enable(&provider.id).unwrap();
        let auth: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(codex_home.join("auth.json")).unwrap())
                .unwrap();
        assert_eq!(auth["tokens"]["access_token"], "chatgpt-oauth");
        assert!(auth.get("OPENAI_API_KEY").is_none());
        let config = fs::read_to_string(codex_home.join("config.toml")).unwrap();
        assert!(config.contains("experimental_bearer_token = \"sk-live-test\""));

        // 切回官方: 统一会话开启后 live 注入共享 custom 路由
        ws.set_unify_codex_session_history(true, false).unwrap();
        let official_id = ws
            .snapshot_for(AppKind::Codex)
            .unwrap()
            .providers
            .iter()
            .find(|p| {
                matches!(
                    p.settings,
                    domain::ProviderSettings::Codex(ref s) if s.kind == CodexKind::Official
                )
            })
            .map(|p| p.id.clone())
            .expect("official provider seeded");
        ws.enable(&official_id).unwrap();
        let config = fs::read_to_string(codex_home.join("config.toml")).unwrap();
        assert!(config.contains("model_provider = \"custom\""));
        assert!(config.contains("supports_websockets = true"));

        // live 未统一前不迁移；编辑表单回填时剥掉统一路由
        let outcome = ws.set_unify_codex_session_history(true, true).unwrap();
        // 当前 live 已注入统一路由，迁移可执行(无历史会话则 0 项)
        assert_eq!(outcome.skipped_reason, None);
        if let ProviderForm::Codex(snapshot) = ws.form_for(&official_id).unwrap() {
            // 官方配置回填后 base_url 仍为空(统一路由表已剥离)
            assert!(snapshot.base_url.is_empty());
        } else {
            panic!("expected codex form");
        }

        // 关闭开关: 清理标记，还原拒绝在开关开启时执行
        ws.set_unify_codex_session_history(false, false).unwrap();
        assert!(!ws.unify_codex_session_history().unwrap());
        ws.set_unify_codex_session_history(true, false).unwrap();
        let restore = ws.restore_codex_unified_history().unwrap();
        assert_eq!(restore.skipped_reason.as_deref(), Some("unify_toggle_on"));
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
            request_protocol: String::new(),
            model_mappings: Vec::new(),
            desktop_mode: None,
        };
        let provider = ws.save_claude_form(None, form).unwrap();
        ws.enable(&provider.id).unwrap();

        let snapshot = ws.snapshot_for(AppKind::Claude).unwrap();
        assert_eq!(snapshot.current_id.as_deref(), Some(provider.id.as_str()));
    }

    #[test]
    fn non_native_protocol_routes_live_config_through_gateway() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("app.db");
        let mut ws = Workspace::open(&db_path, None).unwrap();
        ws.apply_claude_home(Some(temp.path().join("claude")))
            .unwrap();

        // Claude Code natively speaks Anthropic Messages; a Chat Completions
        // provider must be routed through the local compat gateway.
        let form = ClaudeForm {
            name: "ChatOnly".into(),
            website_url: "https://api.example.com".into(),
            kind: ClaudeKind::ThirdParty,
            api_key: "sk-mock-key-12345".into(),
            base_url: "https://api.example.com/v1".into(),
            model: "gpt-5.6-sol".into(),
            request_protocol: "openai-chat".into(),
            model_mappings: Vec::new(),
            desktop_mode: None,
        };
        let routed = ws.save_claude_form(None, form).unwrap();
        ws.enable(&routed.id).unwrap();

        let port = ws.compat_gateway_port().expect("gateway auto-started");
        let target = ws.compat_target_for_app(AppKind::Claude).unwrap();
        assert_eq!(target.base_url, "https://api.example.com/v1");
        assert_eq!(target.protocol, RequestProtocol::OpenAiChat);
        assert_eq!(target.api_key, "sk-mock-key-12345");

        let live = read_claude_live(&ws.claude_paths).unwrap();
        let base_url = live.settings["env"]["ANTHROPIC_BASE_URL"].as_str().unwrap();
        // The provider URL's /v1 path is preserved so the tool's appended
        // endpoint suffix still lands on the gateway.
        assert_eq!(base_url, format!("http://127.0.0.1:{port}/v1"));
        assert_eq!(
            live.settings["env"]["ANTHROPIC_AUTH_TOKEN"],
            "sk-mock-key-12345"
        );

        // Reopening the workspace must bring the gateway back so the tool's
        // already-rewritten live config keeps working.
        drop(ws);
        let reopened = Workspace::open(&db_path, None).unwrap();
        assert!(reopened.compat_gateway_port().is_some());
        assert!(reopened.compat_target_for_app(AppKind::Claude).is_some());
    }

    #[test]
    fn routed_codex_provider_keeps_base_url_path() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("app.db");
        let mut ws = Workspace::open(&db_path, None).unwrap();
        ws.apply_codex_home(Some(temp.path().join("codex")))
            .unwrap();

        // Codex natively speaks Responses; an Anthropic upstream must route
        // through the gateway and keep the /v1 path so Codex's appended
        // `/responses` hits `/v1/responses`.
        let form = CodexForm {
            name: "AnthropicUpstream".into(),
            website_url: "https://api.example.com".into(),
            kind: CodexKind::ResponsesThirdParty,
            api_key: "sk-mock-key-12345".into(),
            base_url: "https://api.example.com/v1".into(),
            model: "claude-sonnet-4-5".into(),
            request_protocol: "anthropic".into(),
            model_mappings: Vec::new(),
        };
        let routed = ws.save_codex_form(None, form).unwrap();
        ws.enable(&routed.id).unwrap();

        let port = ws.compat_gateway_port().expect("gateway auto-started");
        let target = ws.compat_target_for_app(AppKind::Codex).unwrap();
        assert_eq!(target.base_url, "https://api.example.com/v1");
        assert_eq!(target.protocol, RequestProtocol::Anthropic);

        let live = read_codex_live(&ws.codex_paths).unwrap();
        assert!(live
            .config_toml
            .contains(&format!("http://127.0.0.1:{port}/v1")));
        assert!(!live.config_toml.contains("https://api.example.com"));
    }

    #[test]
    fn oauth_status_roundtrip() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("app.db");
        let mut ws = Workspace::open(&db_path, None).unwrap();
        ws.apply_codex_home(Some(temp.path().join("codex")))
            .unwrap();

        // 初始未认证
        assert!(!ws.oauth_status(CODEX_PROVIDER).unwrap().authenticated);
        assert!(!ws.oauth_status(XAI_PROVIDER).unwrap().authenticated);

        let tokens = AuthTokens {
            access_token: "at".into(),
            refresh_token: Some("rt".into()),
            // payload = {"email":"me@example.com"}
            id_token: Some("eyJhbGciOiJIUzI1NiJ9.eyJlbWFpbCI6Im1lQGV4YW1wbGUuY29tIn0.x".into()),
            account_id: Some("acct-1".into()),
            email: Some("me@example.com".into()),
        };

        // Codex: 写原生 auth.json 并可读回
        ws.oauth_complete(CODEX_PROVIDER, &tokens).unwrap();
        let status = ws.oauth_status(CODEX_PROVIDER).unwrap();
        assert!(status.authenticated);
        assert_eq!(status.account.as_deref(), Some("me@example.com"));
        let auth_json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(temp.path().join("codex/auth.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(auth_json["auth_mode"], "chatgpt");
        assert_eq!(auth_json["tokens"]["account_id"], "acct-1");

        // xAI: 存本地 kv
        ws.oauth_complete(XAI_PROVIDER, &tokens).unwrap();
        let status = ws.oauth_status(XAI_PROVIDER).unwrap();
        assert!(status.authenticated);
        assert_eq!(status.account.as_deref(), Some("me@example.com"));

        // 登出清除凭据
        ws.oauth_logout(XAI_PROVIDER).unwrap();
        assert!(!ws.oauth_status(XAI_PROVIDER).unwrap().authenticated);
    }

    #[test]
    fn usage_query_roundtrip_with_mock_upstream() {
        // 本机 mock 余额端点: 任意路径返回 OneAPI 风格余额 JSON
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                use std::io::{Read, Write};
                let mut buf = [0u8; 2048];
                let _ = stream.read(&mut buf);
                let body = br#"{"is_active": true, "balance": 42.0}"#;
                let _ = stream.write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        String::from_utf8_lossy(body)
                    )
                    .as_bytes(),
                );
            }
        });

        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("app.db");
        let mut ws = Workspace::open(&db_path, None).unwrap();
        ws.apply_codex_home(Some(temp.path().join("codex")))
            .unwrap();

        let form = CodexForm {
            name: "BalanceProvider".into(),
            website_url: String::new(),
            kind: CodexKind::ResponsesThirdParty,
            api_key: "sk-mock-key-12345".into(),
            base_url: format!("http://{addr}/v1"),
            model: "gpt-test".into(),
            request_protocol: String::new(),
            model_mappings: Vec::new(),
        };
        let provider = ws.save_codex_form(None, form).unwrap();

        // 未配置时应报错
        assert!(ws.query_provider_usage_blocking(&provider.id).is_err());

        let mut config = domain::UsageScriptConfig::default();
        config.enabled = true;
        config.template_type = domain::TEMPLATE_GENERAL.into();
        config.code = domain::preset_template(domain::TEMPLATE_GENERAL).to_string();
        ws.save_usage_script(&provider.id, &config).unwrap();

        // 默认凭证回退: base_url/api_key 均来自服务商配置
        let result = ws.query_provider_usage_blocking(&provider.id).unwrap();
        assert!(result.success, "result: {result:?}");
        assert_eq!(result.data[0].remaining, Some(42.0));

        // 结果已缓存
        let (cached, _) = ws.usage_result(&provider.id).unwrap().unwrap();
        assert_eq!(cached.data[0].remaining, Some(42.0));

        // 删除配置后不再可查
        ws.delete_usage_script(&provider.id).unwrap();
        assert!(ws.usage_script(&provider.id).unwrap().is_none());
        assert!(ws.query_provider_usage_blocking(&provider.id).is_err());
    }

    #[test]
    fn switching_back_to_native_protocol_restores_direct_connection() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("app.db");
        let mut ws = Workspace::open(&db_path, None).unwrap();
        ws.apply_claude_home(Some(temp.path().join("claude")))
            .unwrap();

        let routed_form = ClaudeForm {
            name: "ChatOnly".into(),
            website_url: "https://api.example.com".into(),
            kind: ClaudeKind::ThirdParty,
            api_key: "sk-mock-key-12345".into(),
            base_url: "https://api.example.com/v1".into(),
            model: "gpt-5.6-sol".into(),
            request_protocol: "openai-chat".into(),
            model_mappings: Vec::new(),
            desktop_mode: None,
        };
        let routed = ws.save_claude_form(None, routed_form).unwrap();
        ws.enable(&routed.id).unwrap();
        assert!(ws.compat_target_for_app(AppKind::Claude).is_some());

        let native_form = ClaudeForm {
            name: "AnthropicNative".into(),
            website_url: "https://api.example.com".into(),
            kind: ClaudeKind::ThirdParty,
            api_key: "sk-ant-test".into(),
            base_url: "https://api.example.com/v1".into(),
            model: "claude-sonnet-4-5".into(),
            request_protocol: "anthropic".into(),
            model_mappings: Vec::new(),
            desktop_mode: None,
        };
        let native = ws.save_claude_form(None, native_form).unwrap();
        ws.enable(&native.id).unwrap();

        assert!(ws.compat_target_for_app(AppKind::Claude).is_none());
        let live = read_claude_live(&ws.claude_paths).unwrap();
        assert_eq!(
            live.settings["env"]["ANTHROPIC_BASE_URL"],
            "https://api.example.com/v1"
        );
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
            request_protocol: String::new(),
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
            request_protocol: String::new(),
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
            request_protocol: String::new(),
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
            request_protocol: String::new(),
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
            default_reasoning_effort: "high".into(),
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

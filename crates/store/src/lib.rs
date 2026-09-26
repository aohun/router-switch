//! SQLite SSOT under ~/.router-switch/app.db

use std::path::{Path, PathBuf};

use domain::{
    official_claude_desktop_provider, official_claude_provider, official_codex_provider,
    official_cursor_provider, official_grok_provider, official_opencode_provider,
    official_pi_provider, official_workbuddy_provider, official_zcode_provider, AppKind, Prompt,
    Provider, OFFICIAL_CLAUDE_DESKTOP_ID, OFFICIAL_CLAUDE_ID, OFFICIAL_CODEX_ID,
    OFFICIAL_CURSOR_ID, OFFICIAL_GROK_ID, OFFICIAL_OPENCODE_ID, OFFICIAL_PI_ID,
    OFFICIAL_WORKBUDDY_ID, OFFICIAL_ZCODE_ID,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("无法解析用户主目录")]
    HomeDir,
    #[error("数据库错误: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("服务商数据损坏: {0}")]
    Corrupt(String),
    #[error("{0}")]
    Conflict(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum AppLanguage {
    #[default]
    ZhCn,
    En,
}

impl AppLanguage {
    pub fn locale_str(&self) -> &'static str {
        match self {
            Self::ZhCn => "zh-CN",
            Self::En => "en",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_log_level")]
    pub level: LogLevel,
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,
}

fn default_true() -> bool {
    true
}

fn default_log_level() -> LogLevel {
    LogLevel::Info
}

fn default_retention_days() -> u32 {
    7
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            level: LogLevel::Info,
            retention_days: 7,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Trace => "trace",
        }
    }

    pub fn priority(&self) -> u8 {
        match self {
            Self::Error => 1,
            Self::Warn => 2,
            Self::Info => 3,
            Self::Debug => 4,
            Self::Trace => 5,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppSettings {
    pub codex_home: Option<PathBuf>,
    pub claude_home: Option<PathBuf>,
    #[serde(default)]
    pub claude_desktop_home: Option<PathBuf>,
    pub grok_home: Option<PathBuf>,
    pub opencode_home: Option<PathBuf>,
    pub pi_home: Option<PathBuf>,
    pub cursor_home: Option<PathBuf>,
    pub zcode_home: Option<PathBuf>,
    pub workbuddy_home: Option<PathBuf>,
    pub theme: ThemePreference,
    #[serde(default)]
    pub language: AppLanguage,
    #[serde(default = "default_main_apps")]
    pub main_apps: Vec<String>,
    #[serde(default)]
    pub launch_on_startup: bool,
    #[serde(default = "default_minimize_to_tray")]
    pub minimize_to_tray: bool,
    #[serde(default = "default_auto_check_update")]
    pub auto_check_update: bool,
    #[serde(default)]
    pub skipped_update_version: Option<String>,
    #[serde(default)]
    pub log_config: LogConfig,
    /// 非接管切换第三方 Codex 供应商时保留 auth.json 的官方登录
    #[serde(default)]
    pub preserve_codex_official_auth_on_switch: bool,
    /// 统一 Codex 会话历史: 官方订阅以共享 custom 供应商标识运行
    #[serde(default)]
    pub unify_codex_session_history: bool,
    /// 统一会话开启时是否迁入现有官方会话(迁移前自动备份)
    #[serde(default)]
    pub unify_codex_migrate_existing: bool,
    /// 官方会话统一迁移完成标记(按 Codex 目录绑定)
    #[serde(default)]
    pub codex_official_history_unify: Option<CodexUnifyMigrationMarker>,
}

/// 官方会话统一迁移的完成账本
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodexUnifyMigrationMarker {
    #[serde(rename = "completedAt")]
    pub completed_at: String,
    #[serde(rename = "codexConfigDir")]
    pub codex_config_dir: String,
    #[serde(rename = "migratedJsonlFiles", default)]
    pub migrated_jsonl_files: u64,
    #[serde(rename = "migratedStateRows", default)]
    pub migrated_state_rows: u64,
}

fn default_main_apps() -> Vec<String> {
    vec![
        "claude".into(),
        "codex".into(),
        "claude-desktop".into(),
        "cursor".into(),
        "opencode".into(),
        "grok".into(),
        "pi".into(),
        "zcode".into(),
        "workbuddy".into(),
    ]
}

fn default_minimize_to_tray() -> bool {
    true
}

fn default_auto_check_update() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemePreference {
    System,
    Light,
    Dark,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            codex_home: None,
            claude_home: None,
            claude_desktop_home: None,
            grok_home: None,
            opencode_home: None,
            pi_home: None,
            cursor_home: None,
            zcode_home: None,
            workbuddy_home: None,
            theme: ThemePreference::System,
            language: AppLanguage::ZhCn,
            main_apps: default_main_apps(),
            launch_on_startup: false,
            minimize_to_tray: true,
            auto_check_update: true,
            skipped_update_version: None,
            log_config: LogConfig::default(),
            preserve_codex_official_auth_on_switch: false,
            unify_codex_session_history: false,
            unify_codex_migrate_existing: false,
            codex_official_history_unify: None,
        }
    }
}

pub struct Store {
    conn: Connection,
    data_dir: PathBuf,
}

impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| {
                StoreError::Corrupt(format!("无法创建数据目录 {}: {err}", parent.display()))
            })?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            CREATE TABLE IF NOT EXISTS providers (
                id TEXT PRIMARY KEY,
                app TEXT NOT NULL,
                name TEXT NOT NULL,
                website_url TEXT,
                settings_json TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                sort_index INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS current_providers (
                app TEXT PRIMARY KEY,
                provider_id TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS kv (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS provider_usage_scripts (
                provider_id TEXT PRIMARY KEY,
                config_json TEXT NOT NULL,
                result_json TEXT,
                fetched_at INTEGER
            );
            CREATE TABLE IF NOT EXISTS prompts (
                id TEXT NOT NULL,
                app TEXT NOT NULL,
                name TEXT NOT NULL,
                content TEXT NOT NULL,
                description TEXT,
                enabled BOOLEAN NOT NULL DEFAULT 0,
                created_at INTEGER,
                updated_at INTEGER,
                PRIMARY KEY (id, app)
            );
            CREATE TABLE IF NOT EXISTS session_user_data (
                session_key TEXT PRIMARY KEY,
                favorite INTEGER NOT NULL DEFAULT 0,
                pinned INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS session_index (
                session_key TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                session_id TEXT NOT NULL,
                title TEXT,
                project_name TEXT,
                source_path TEXT,
                created_at INTEGER,
                updated_at INTEGER,
                message_count INTEGER NOT NULL DEFAULT 0,
                tokens INTEGER NOT NULL DEFAULT 0
            );
            ",
        )?;
        let data_dir = path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let store = Self { conn, data_dir };
        store.seed_official_providers()?;
        Ok(store)
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn list_providers(&self, app: AppKind) -> Result<Vec<Provider>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, app, name, website_url, settings_json, created_at, sort_index
             FROM providers WHERE app = ?1
             ORDER BY sort_index ASC, created_at ASC",
        )?;
        let rows = stmt.query_map(params![app.as_str()], |row| {
            Ok(Row {
                id: row.get(0)?,
                app: row.get(1)?,
                name: row.get(2)?,
                website_url: row.get(3)?,
                settings_json: row.get(4)?,
                created_at: row.get(5)?,
                sort_index: row.get(6)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?.into_provider()?);
        }
        Ok(out)
    }

    pub fn get_provider(&self, id: &str) -> Result<Option<Provider>, StoreError> {
        self.conn
            .query_row(
                "SELECT id, app, name, website_url, settings_json, created_at, sort_index
                 FROM providers WHERE id = ?1",
                params![id],
                |row| {
                    Ok(Row {
                        id: row.get(0)?,
                        app: row.get(1)?,
                        name: row.get(2)?,
                        website_url: row.get(3)?,
                        settings_json: row.get(4)?,
                        created_at: row.get(5)?,
                        sort_index: row.get(6)?,
                    })
                },
            )
            .optional()?
            .map(Row::into_provider)
            .transpose()
    }

    pub fn upsert_provider(&self, provider: &Provider) -> Result<(), StoreError> {
        let settings_json = serde_json::to_string(&provider.settings)
            .map_err(|err| StoreError::Corrupt(err.to_string()))?;
        self.conn.execute(
            "INSERT INTO providers (id, app, name, website_url, settings_json, created_at, sort_index)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
                app = excluded.app,
                name = excluded.name,
                website_url = excluded.website_url,
                settings_json = excluded.settings_json,
                sort_index = excluded.sort_index",
            params![
                provider.id,
                provider.app.as_str(),
                provider.name,
                provider.website_url,
                settings_json,
                provider.created_at,
                provider.sort_index,
            ],
        )?;
        Ok(())
    }

    pub fn delete_provider(&self, id: &str) -> Result<(), StoreError> {
        if self.is_current(id)? {
            return Err(StoreError::Conflict("当前启用的服务商不能删除".into()));
        }
        self.conn
            .execute("DELETE FROM providers WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn current_id(&self, app: AppKind) -> Result<Option<String>, StoreError> {
        self.conn
            .query_row(
                "SELECT provider_id FROM current_providers WHERE app = ?1",
                params![app.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(StoreError::from)
    }

    pub fn set_current(&self, app: AppKind, provider_id: &str) -> Result<(), StoreError> {
        let exists: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM providers WHERE id = ?1 AND app = ?2",
                params![provider_id, app.as_str()],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_none() {
            return Err(StoreError::Conflict("服务商不存在".into()));
        }
        self.conn.execute(
            "INSERT INTO current_providers (app, provider_id) VALUES (?1, ?2)
             ON CONFLICT(app) DO UPDATE SET provider_id = excluded.provider_id",
            params![app.as_str(), provider_id],
        )?;
        Ok(())
    }

    pub fn settings(&self) -> Result<AppSettings, StoreError> {
        let raw: Option<String> = self
            .conn
            .query_row("SELECT value FROM kv WHERE key = 'settings'", [], |row| {
                row.get(0)
            })
            .optional()?;
        let mut settings = match raw {
            Some(text) => serde_json::from_str(&text)
                .map_err(|err| StoreError::Corrupt(format!("settings: {err}")))?,
            None => AppSettings::default(),
        };
        self.migrate_main_apps_once(&mut settings)?;
        Ok(settings)
    }

    /// 老库一次性迁移: 把后新增的应用加入主页面显示(用户随后可自行隐藏)。
    fn migrate_main_apps_once(&self, settings: &mut AppSettings) -> Result<(), StoreError> {
        const KEY: &str = "migrated-main-apps-v2";
        let done: Option<String> = self
            .conn
            .query_row("SELECT value FROM kv WHERE key = ?1", [KEY], |row| {
                row.get(0)
            })
            .optional()?;
        if done.is_some() {
            return Ok(());
        }
        let mut changed = false;
        for app in ["claude-desktop"] {
            if !settings.main_apps.iter().any(|item| item == app) {
                settings.main_apps.push(app.to_string());
                changed = true;
            }
        }
        self.conn.execute(
            "INSERT INTO kv (key, value) VALUES (?1, '1')
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [KEY],
        )?;
        if changed {
            self.save_settings(settings)?;
        }
        Ok(())
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<(), StoreError> {
        let value =
            serde_json::to_string(settings).map_err(|err| StoreError::Corrupt(err.to_string()))?;
        self.conn.execute(
            "INSERT INTO kv (key, value) VALUES ('settings', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![value],
        )?;
        Ok(())
    }

    /// 通用 KV 写入(认证凭据等)
    pub fn kv_set(&self, key: &str, value: &str) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT INTO kv (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                rusqlite::params![key, value],
            )
            .map_err(StoreError::from)?;
        Ok(())
    }

    /// 通用 KV 读取
    pub fn kv_get(&self, key: &str) -> Result<Option<String>, StoreError> {
        let mut stmt = self.conn.prepare("SELECT value FROM kv WHERE key = ?1")?;
        let mut rows = stmt.query(rusqlite::params![key])?;
        if let Some(row) = rows.next()? {
            return Ok(Some(row.get(0)?));
        }
        Ok(None)
    }

    pub fn kv_delete(&self, key: &str) -> Result<(), StoreError> {
        self.conn
            .execute("DELETE FROM kv WHERE key = ?1", rusqlite::params![key])
            .map_err(StoreError::from)?;
        Ok(())
    }

    /// 保存服务商的用量查询配置(保留已缓存的结果)
    pub fn save_usage_script(
        &self,
        provider_id: &str,
        config: &domain::UsageScriptConfig,
    ) -> Result<(), StoreError> {
        let config_json = serde_json::to_string(config)
            .map_err(|e| StoreError::Corrupt(format!("用量脚本配置序列化失败: {e}")))?;
        self.conn
            .execute(
                "INSERT INTO provider_usage_scripts (provider_id, config_json, result_json, fetched_at)
                 VALUES (?1, ?2, NULL, NULL)
                 ON CONFLICT(provider_id) DO UPDATE SET config_json = excluded.config_json",
                rusqlite::params![provider_id, config_json],
            )
            .map_err(StoreError::from)?;
        Ok(())
    }

    pub fn usage_script(
        &self,
        provider_id: &str,
    ) -> Result<Option<domain::UsageScriptConfig>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT config_json FROM provider_usage_scripts WHERE provider_id = ?1")?;
        let mut rows = stmt.query(rusqlite::params![provider_id])?;
        if let Some(row) = rows.next()? {
            let json: String = row.get(0)?;
            return Ok(Some(serde_json::from_str(&json).map_err(|e| {
                StoreError::Corrupt(format!("用量脚本配置损坏: {e}"))
            })?));
        }
        Ok(None)
    }

    pub fn delete_usage_script(&self, provider_id: &str) -> Result<(), StoreError> {
        self.conn
            .execute(
                "DELETE FROM provider_usage_scripts WHERE provider_id = ?1",
                rusqlite::params![provider_id],
            )
            .map_err(StoreError::from)?;
        Ok(())
    }

    /// 缓存最近一次查询结果
    pub fn save_usage_result(
        &self,
        provider_id: &str,
        result: &domain::UsageQueryResult,
        fetched_at: i64,
    ) -> Result<(), StoreError> {
        let result_json = serde_json::to_string(result)
            .map_err(|e| StoreError::Corrupt(format!("用量结果序列化失败: {e}")))?;
        self.conn
            .execute(
                "UPDATE provider_usage_scripts SET result_json = ?2, fetched_at = ?3
                 WHERE provider_id = ?1",
                rusqlite::params![provider_id, result_json, fetched_at],
            )
            .map_err(StoreError::from)?;
        Ok(())
    }

    pub fn usage_result(
        &self,
        provider_id: &str,
    ) -> Result<Option<(domain::UsageQueryResult, i64)>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT result_json, fetched_at FROM provider_usage_scripts
             WHERE provider_id = ?1 AND result_json IS NOT NULL",
        )?;
        let mut rows = stmt.query(rusqlite::params![provider_id])?;
        if let Some(row) = rows.next()? {
            let json: String = row.get(0)?;
            let fetched_at: i64 = row.get(1)?;
            return Ok(Some((
                serde_json::from_str(&json)
                    .map_err(|e| StoreError::Corrupt(format!("用量结果损坏: {e}")))?,
                fetched_at,
            )));
        }
        Ok(None)
    }

    /// 全量列出用量脚本配置(自动刷新调度用): (provider_id, config, fetched_at)
    pub fn list_usage_scripts(
        &self,
    ) -> Result<Vec<(String, domain::UsageScriptConfig, Option<i64>)>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT provider_id, config_json, fetched_at FROM provider_usage_scripts")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<i64>>(2)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (provider_id, json, fetched_at) = row?;
            let config: domain::UsageScriptConfig = serde_json::from_str(&json)
                .map_err(|e| StoreError::Corrupt(format!("用量脚本配置损坏: {e}")))?;
            out.push((provider_id, config, fetched_at));
        }
        Ok(out)
    }

    /// 获取指定应用的所有提示词
    pub fn list_prompts(&self, app: AppKind) -> Result<Vec<Prompt>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, content, description, enabled, created_at, updated_at
             FROM prompts WHERE app = ?1
             ORDER BY created_at ASC, id ASC",
        )?;
        let rows = stmt.query_map(params![app.as_str()], |row| {
            Ok(Prompt {
                id: row.get(0)?,
                name: row.get(1)?,
                content: row.get(2)?,
                description: row.get(3)?,
                enabled: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// 获取指定提示词
    pub fn get_prompt(&self, app: AppKind, id: &str) -> Result<Option<Prompt>, StoreError> {
        self.conn
            .query_row(
                "SELECT id, name, content, description, enabled, created_at, updated_at
                 FROM prompts WHERE app = ?1 AND id = ?2",
                params![app.as_str(), id],
                |row| {
                    Ok(Prompt {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        content: row.get(2)?,
                        description: row.get(3)?,
                        enabled: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    })
                },
            )
            .optional()
            .map_err(StoreError::from)
    }

    /// 保存或更新提示词
    pub fn upsert_prompt(&self, app: AppKind, prompt: &Prompt) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO prompts (id, app, name, content, description, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id, app) DO UPDATE SET
                name = excluded.name,
                content = excluded.content,
                description = excluded.description,
                enabled = excluded.enabled,
                created_at = excluded.created_at,
                updated_at = excluded.updated_at",
            params![
                prompt.id,
                app.as_str(),
                prompt.name,
                prompt.content,
                prompt.description,
                prompt.enabled,
                prompt.created_at,
                prompt.updated_at,
            ],
        )?;
        Ok(())
    }

    /// 删除指定提示词
    pub fn delete_prompt(&self, app: AppKind, id: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "DELETE FROM prompts WHERE app = ?1 AND id = ?2",
            params![app.as_str(), id],
        )?;
        Ok(())
    }

    /// 切换指定提示词的启用状态
    pub fn set_prompt_enabled(
        &self,
        app: AppKind,
        id: &str,
        enabled: bool,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE prompts SET enabled = ?3, updated_at = ?4 WHERE app = ?1 AND id = ?2",
            params![app.as_str(), id, enabled, now_secs()],
        )?;
        Ok(())
    }

    /// 禁用指定应用的所有提示词
    pub fn disable_all_prompts(&self, app: AppKind) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE prompts SET enabled = 0, updated_at = ?2 WHERE app = ?1",
            params![app.as_str(), now_secs()],
        )?;
        Ok(())
    }

    fn is_current(&self, id: &str) -> Result<bool, StoreError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM current_providers WHERE provider_id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    fn seed_official_providers(&self) -> Result<(), StoreError> {
        // Seed Codex
        if self.get_provider(OFFICIAL_CODEX_ID)?.is_none() {
            let mut official = official_codex_provider();
            official.created_at = now_secs();
            self.upsert_provider(&official)?;
        }
        // Seed Claude
        if self.get_provider(OFFICIAL_CLAUDE_ID)?.is_none() {
            let mut official = official_claude_provider();
            official.created_at = now_secs();
            self.upsert_provider(&official)?;
        }
        // Seed Claude Desktop
        if self.get_provider(OFFICIAL_CLAUDE_DESKTOP_ID)?.is_none() {
            let mut official = official_claude_desktop_provider();
            official.created_at = now_secs();
            self.upsert_provider(&official)?;
        }
        // Seed Grok
        if self.get_provider(OFFICIAL_GROK_ID)?.is_none() {
            let mut official = official_grok_provider();
            official.created_at = now_secs();
            self.upsert_provider(&official)?;
        }
        // Seed OpenCode
        if self.get_provider(OFFICIAL_OPENCODE_ID)?.is_none() {
            let mut official = official_opencode_provider();
            official.created_at = now_secs();
            self.upsert_provider(&official)?;
        }
        // Seed Pi
        if self.get_provider(OFFICIAL_PI_ID)?.is_none() {
            let mut official = official_pi_provider();
            official.created_at = now_secs();
            self.upsert_provider(&official)?;
        }
        // Seed Cursor
        if self.get_provider(OFFICIAL_CURSOR_ID)?.is_none() {
            let mut official = official_cursor_provider();
            official.created_at = now_secs();
            self.upsert_provider(&official)?;
        }
        // Seed ZCode
        if self.get_provider(OFFICIAL_ZCODE_ID)?.is_none() {
            let mut official = official_zcode_provider();
            official.created_at = now_secs();
            self.upsert_provider(&official)?;
        }
        // Seed WorkBuddy
        if self.get_provider(OFFICIAL_WORKBUDDY_ID)?.is_none() {
            let mut official = official_workbuddy_provider();
            official.created_at = now_secs();
            self.upsert_provider(&official)?;
        }
        Ok(())
    }

    // ---- Wake-style session index + user_data (favorite / pinned) ----

    pub fn replace_session_index(
        &self,
        rows: &[SessionIndexRow],
    ) -> Result<(), StoreError> {
        self.conn.execute("DELETE FROM session_index", [])?;
        let mut stmt = self.conn.prepare(
            "INSERT INTO session_index
             (session_key, agent_id, session_id, title, project_name, source_path,
              created_at, updated_at, message_count, tokens)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        )?;
        for row in rows {
            stmt.execute(params![
                row.session_key,
                row.agent_id,
                row.session_id,
                row.title,
                row.project_name,
                row.source_path,
                row.created_at,
                row.updated_at,
                row.message_count,
                row.tokens,
            ])?;
        }
        Ok(())
    }

    pub fn list_session_index(&self) -> Result<Vec<SessionIndexRow>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT session_key, agent_id, session_id, title, project_name, source_path,
                    created_at, updated_at, message_count, tokens
             FROM session_index
             ORDER BY COALESCE(updated_at, created_at, 0) DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(SessionIndexRow {
                session_key: row.get(0)?,
                agent_id: row.get(1)?,
                session_id: row.get(2)?,
                title: row.get(3)?,
                project_name: row.get(4)?,
                source_path: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                message_count: row.get(8)?,
                tokens: row.get(9)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }

    pub fn get_session_user_data(
        &self,
        session_key: &str,
    ) -> Result<SessionUserData, StoreError> {
        let row = self
            .conn
            .query_row(
                "SELECT favorite, pinned FROM session_user_data WHERE session_key = ?1",
                params![session_key],
                |row| {
                    Ok(SessionUserData {
                        favorite: row.get::<_, i64>(0)? != 0,
                        pinned: row.get::<_, i64>(1)? != 0,
                    })
                },
            )
            .optional()?;
        Ok(row.unwrap_or_default())
    }

    pub fn list_session_user_data(&self) -> Result<Vec<(String, SessionUserData)>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT session_key, favorite, pinned FROM session_user_data")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                SessionUserData {
                    favorite: row.get::<_, i64>(1)? != 0,
                    pinned: row.get::<_, i64>(2)? != 0,
                },
            ))
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }

    pub fn set_session_favorite(
        &self,
        session_key: &str,
        favorite: bool,
    ) -> Result<(), StoreError> {
        self.upsert_session_user_flag(session_key, Some(favorite), None)
    }

    pub fn set_session_pinned(&self, session_key: &str, pinned: bool) -> Result<(), StoreError> {
        self.upsert_session_user_flag(session_key, None, Some(pinned))
    }

    fn upsert_session_user_flag(
        &self,
        session_key: &str,
        favorite: Option<bool>,
        pinned: Option<bool>,
    ) -> Result<(), StoreError> {
        let current = self.get_session_user_data(session_key)?;
        let favorite = favorite.unwrap_or(current.favorite);
        let pinned = pinned.unwrap_or(current.pinned);
        if !favorite && !pinned {
            self.conn.execute(
                "DELETE FROM session_user_data WHERE session_key = ?1",
                params![session_key],
            )?;
        } else {
            self.conn.execute(
                "INSERT INTO session_user_data (session_key, favorite, pinned)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(session_key) DO UPDATE SET
                   favorite = excluded.favorite,
                   pinned = excluded.pinned",
                params![session_key, favorite as i64, pinned as i64],
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionUserData {
    pub favorite: bool,
    pub pinned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionIndexRow {
    pub session_key: String,
    pub agent_id: String,
    pub session_id: String,
    pub title: Option<String>,
    pub project_name: Option<String>,
    pub source_path: Option<String>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
    pub message_count: i64,
    pub tokens: i64,
}

pub fn default_data_dir() -> Result<PathBuf, StoreError> {
    if let Ok(home) = std::env::var("ROUTER_SWITCH_HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }
    let home = dirs::home_dir().ok_or(StoreError::HomeDir)?;
    Ok(home.join(".router-switch"))
}

pub fn default_db_path() -> Result<PathBuf, StoreError> {
    Ok(default_data_dir()?.join("app.db"))
}

struct Row {
    id: String,
    app: String,
    name: String,
    website_url: Option<String>,
    settings_json: String,
    created_at: i64,
    sort_index: i64,
}

impl Row {
    fn into_provider(self) -> Result<Provider, StoreError> {
        let app = AppKind::parse(&self.app)
            .ok_or_else(|| StoreError::Corrupt(format!("未知应用 {}", self.app)))?;
        let settings = serde_json::from_str(&self.settings_json)
            .map_err(|err| StoreError::Corrupt(format!("{}: {err}", self.id)))?;
        Ok(Provider {
            id: self.id,
            app,
            name: self.name,
            website_url: self.website_url,
            settings,
            created_at: self.created_at,
            sort_index: self.sort_index,
        })
    }
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
    use domain::{
        generate_third_party_auth, generate_third_party_config, new_provider_id, CodexKind,
        CodexSettings, OFFICIAL_CLAUDE_ID, OFFICIAL_CODEX_ID, OFFICIAL_GROK_ID,
    };

    fn temp_store() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path().join("app.db")).unwrap();
        (dir, store)
    }

    #[test]
    fn seeds_official_and_blocks_deleting_current() {
        let (_dir, store) = temp_store();
        let codex_list = store.list_providers(AppKind::Codex).unwrap();
        assert_eq!(codex_list.len(), 1);
        assert_eq!(codex_list[0].id, OFFICIAL_CODEX_ID);

        let claude_list = store.list_providers(AppKind::Claude).unwrap();
        assert_eq!(claude_list.len(), 1);
        assert_eq!(claude_list[0].id, OFFICIAL_CLAUDE_ID);

        let grok_list = store.list_providers(AppKind::Grok).unwrap();
        assert_eq!(grok_list.len(), 1);
        assert_eq!(grok_list[0].id, OFFICIAL_GROK_ID);

        let pi_list = store.list_providers(AppKind::Pi).unwrap();
        assert_eq!(pi_list.len(), 1);
        assert_eq!(pi_list[0].id, OFFICIAL_PI_ID);

        let cursor_list = store.list_providers(AppKind::Cursor).unwrap();
        assert_eq!(cursor_list.len(), 1);
        assert_eq!(cursor_list[0].id, OFFICIAL_CURSOR_ID);

        let zcode_list = store.list_providers(AppKind::ZCode).unwrap();
        assert_eq!(zcode_list.len(), 1);
        assert_eq!(zcode_list[0].id, OFFICIAL_ZCODE_ID);

        let workbuddy_list = store.list_providers(AppKind::WorkBuddy).unwrap();
        assert_eq!(workbuddy_list.len(), 1);
        assert_eq!(workbuddy_list[0].id, OFFICIAL_WORKBUDDY_ID);

        store
            .set_current(AppKind::Codex, OFFICIAL_CODEX_ID)
            .unwrap();
        let err = store.delete_provider(OFFICIAL_CODEX_ID).unwrap_err();
        assert!(matches!(err, StoreError::Conflict(_)));
    }

    #[test]
    fn upsert_and_delete_third_party() {
        let (_dir, store) = temp_store();
        let provider = Provider {
            id: new_provider_id("packy"),
            app: AppKind::Codex,
            name: "PackyCode".into(),
            website_url: Some("https://www.packyapi.ai".into()),
            settings: domain::ProviderSettings::Codex(CodexSettings {
                kind: CodexKind::ResponsesThirdParty,
                auth: generate_third_party_auth("sk-test"),
                config_toml: generate_third_party_config(
                    "PackyCode",
                    "https://www.packyapi.ai/v1",
                    "gpt-5.6-sol",
                ),
                request_protocol: String::new(),
                model_mappings: Vec::new(),
            }),
            created_at: 1,
            sort_index: 1,
        };
        store.upsert_provider(&provider).unwrap();
        assert_eq!(store.list_providers(AppKind::Codex).unwrap().len(), 2);
        store.delete_provider(&provider.id).unwrap();
        assert_eq!(store.list_providers(AppKind::Codex).unwrap().len(), 1);
    }

    #[test]
    fn prompt_crud_operations() {
        let (_dir, store) = temp_store();
        assert!(store.list_prompts(AppKind::Codex).unwrap().is_empty());

        let p1 = Prompt {
            id: "p1".into(),
            name: "Default Codex".into(),
            content: "You are a helpful coding assistant.".into(),
            description: Some("Main prompt".into()),
            enabled: true,
            created_at: Some(100),
            updated_at: Some(100),
        };
        store.upsert_prompt(AppKind::Codex, &p1).unwrap();

        let list = store.list_prompts(AppKind::Codex).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "p1");
        assert_eq!(list[0].name, "Default Codex");
        assert!(list[0].enabled);

        // Fetch single
        let fetched = store.get_prompt(AppKind::Codex, "p1").unwrap().unwrap();
        assert_eq!(fetched.content, "You are a helpful coding assistant.");

        // Disable
        store
            .set_prompt_enabled(AppKind::Codex, "p1", false)
            .unwrap();
        assert!(
            !store
                .get_prompt(AppKind::Codex, "p1")
                .unwrap()
                .unwrap()
                .enabled
        );

        // Delete
        store.delete_prompt(AppKind::Codex, "p1").unwrap();
        assert!(store.list_prompts(AppKind::Codex).unwrap().is_empty());
    }
}

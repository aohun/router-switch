//! SQLite SSOT under ~/.router-switch/app.db

use std::path::{Path, PathBuf};

use domain::{
    official_claude_provider, official_codex_provider, official_cursor_provider,
    official_grok_provider, official_opencode_provider, official_pi_provider,
    official_workbuddy_provider, official_zcode_provider, AppKind, Provider, OFFICIAL_CLAUDE_ID,
    OFFICIAL_CODEX_ID, OFFICIAL_CURSOR_ID, OFFICIAL_GROK_ID, OFFICIAL_OPENCODE_ID, OFFICIAL_PI_ID,
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
    #[serde(default)]
    pub log_config: LogConfig,
}

fn default_main_apps() -> Vec<String> {
    vec![
        "codex".into(),
        "claude".into(),
        "claude-desktop".into(),
        "grok".into(),
        "opencode".into(),
        "pi".into(),
        "cursor".into(),
        "zcode".into(),
        "workbuddy".into(),
    ]
}

fn default_minimize_to_tray() -> bool {
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
            log_config: LogConfig::default(),
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
        match raw {
            Some(text) => serde_json::from_str(&text)
                .map_err(|err| StoreError::Corrupt(format!("settings: {err}"))),
            None => Ok(AppSettings::default()),
        }
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
}

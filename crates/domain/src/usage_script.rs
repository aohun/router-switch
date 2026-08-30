//! 用量查询脚本:配置模型、预设模板与结果类型(对齐 cc-switch 的 UsageScript 设计)。
//!
//! 脚本本体是一段返回 `({ request: {...}, extractor: function(response) {...} })`
//! 的 JavaScript,由 `usage-query` crate 在受控 QuickJS 运行时里执行:
//! 先 eval 取出 `request` 发 HTTP,再把响应交给 `extractor` 得到用量字段。

use serde::{Deserialize, Serialize};

pub const TEMPLATE_CUSTOM: &str = "custom";
pub const TEMPLATE_GENERAL: &str = "general";
pub const TEMPLATE_NEW_API: &str = "new_api";

fn default_template() -> String {
    TEMPLATE_GENERAL.to_string()
}

fn default_timeout_secs() -> u64 {
    10
}

fn default_auto_interval_minutes() -> u32 {
    5
}

/// 每个服务商的用量查询配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageScriptConfig {
    pub enabled: bool,
    #[serde(default = "default_template")]
    pub template_type: String,
    #[serde(default)]
    pub code: String,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    /// 自动查询间隔(分钟), 0 表示不自动查询
    #[serde(default = "default_auto_interval_minutes")]
    pub auto_interval_minutes: u32,
    /// 用量查询专用 API Key;留空回退服务商自身的 Key
    #[serde(default)]
    pub api_key: Option<String>,
    /// 用量查询专用请求地址;留空回退服务商自身的 Base URL
    #[serde(default)]
    pub base_url: Option<String>,
    /// NewAPI 模板的访问令牌 ({{accessToken}})
    #[serde(default)]
    pub access_token: Option<String>,
    /// NewAPI 模板的用户 ID ({{userId}})
    #[serde(default)]
    pub user_id: Option<String>,
}

impl Default for UsageScriptConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            template_type: default_template(),
            code: String::new(),
            timeout_secs: default_timeout_secs(),
            auto_interval_minutes: default_auto_interval_minutes(),
            api_key: None,
            base_url: None,
            access_token: None,
            user_id: None,
        }
    }
}

impl UsageScriptConfig {
    /// 填入模板默认代码(模板切换或代码为空时使用)
    pub fn with_preset_code(mut self) -> Self {
        if self.code.trim().is_empty() {
            self.code = preset_template(&self.template_type).to_string();
        }
        self
    }

    fn non_empty(value: &Option<String>) -> Option<&str> {
        value.as_deref().map(str::trim).filter(|s| !s.is_empty())
    }

    /// 查询用 API Key:专用值优先,否则回退服务商默认
    pub fn effective_api_key<'a>(&'a self, provider_default: &'a str) -> &'a str {
        Self::non_empty(&self.api_key).unwrap_or(provider_default)
    }

    /// 查询用请求地址:专用值优先,否则回退服务商默认
    pub fn effective_base_url<'a>(&'a self, provider_default: &'a str) -> &'a str {
        Self::non_empty(&self.base_url).unwrap_or(provider_default)
    }
}

/// 单条用量数据(extractor 返回对象或对象数组)
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageDataItem {
    #[serde(default)]
    pub plan_name: Option<String>,
    #[serde(default)]
    pub extra: Option<String>,
    #[serde(default)]
    pub is_valid: Option<bool>,
    #[serde(default)]
    pub invalid_message: Option<String>,
    #[serde(default)]
    pub total: Option<f64>,
    #[serde(default)]
    pub used: Option<f64>,
    #[serde(default)]
    pub remaining: Option<f64>,
    #[serde(default)]
    pub unit: Option<String>,
}

/// 用量查询结果
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UsageQueryResult {
    pub success: bool,
    #[serde(default)]
    pub data: Vec<UsageDataItem>,
    #[serde(default)]
    pub error: Option<String>,
}

/// 把执行器返回的原始 JSON(已通过校验)转成结果模型;支持单对象或数组
pub fn parse_usage_result(value: serde_json::Value) -> UsageQueryResult {
    let items: Vec<serde_json::Value> = match value {
        serde_json::Value::Array(items) => items,
        item @ serde_json::Value::Object(_) => vec![item],
        _ => return UsageQueryResult::default(),
    };
    let data = items
        .into_iter()
        .filter_map(|item| serde_json::from_value(item).ok())
        .collect();
    UsageQueryResult {
        success: true,
        data,
        error: None,
    }
}

/// 失败结果(保留上次错误用于卡片展示)
pub fn failed_usage_result(message: impl Into<String>) -> UsageQueryResult {
    UsageQueryResult {
        success: false,
        data: Vec::new(),
        error: Some(message.into()),
    }
}

/// 模板展示名
pub fn template_display_name(template_type: &str) -> &'static str {
    match template_type {
        TEMPLATE_CUSTOM => "自定义",
        TEMPLATE_NEW_API => "NewAPI",
        _ => "通用模板",
    }
}

/// 预设模板 JS 代码(移植自 cc-switch)
pub fn preset_template(template_type: &str) -> &'static str {
    match template_type {
        TEMPLATE_CUSTOM => {
            r#"({
  request: {
    url: "",
    method: "GET",
    headers: {}
  },
  extractor: function(response) {
    return {
      remaining: 0,
      unit: "USD"
    };
  }
})"#
        }
        TEMPLATE_NEW_API => {
            r#"({
  request: {
    url: "{{baseUrl}}/api/user/self",
    method: "GET",
    headers: {
      "Content-Type": "application/json",
      "Authorization": "Bearer {{accessToken}}",
      "User-Agent": "router-switch/1.0",
      "New-Api-User": "{{userId}}"
    }
  },
  extractor: function(response) {
    if (response.success && response.data) {
      return {
        planName: response.data.group || "默认分组",
        remaining: response.data.quota / 500000,
        used: response.data.used_quota / 500000,
        total: (response.data.quota + response.data.used_quota) / 500000,
        unit: "USD"
      };
    }
    return {
      isValid: false,
      invalidMessage: response.message || "查询失败"
    };
  }
})"#
        }
        // TEMPLATE_GENERAL 及未识别类型的兜底
        _ => {
            r#"({
  request: {
    url: "{{baseUrl}}/user/balance",
    method: "GET",
    headers: {
      "Authorization": "Bearer {{apiKey}}",
      "User-Agent": "router-switch/1.0"
    }
  },
  extractor: function(response) {
    return {
      isValid: response.is_active || true,
      remaining: response.balance,
      unit: "USD"
    };
  }
})"#
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults_roundtrip() {
        let config = UsageScriptConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let back: UsageScriptConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back, config);
        assert_eq!(back.template_type, TEMPLATE_GENERAL);
        assert_eq!(back.timeout_secs, 10);
        assert_eq!(back.auto_interval_minutes, 5);
    }

    #[test]
    fn effective_values_fall_back_to_provider_defaults() {
        let mut config = UsageScriptConfig::default();
        config.api_key = Some("  ".into());
        config.base_url = Some("https://api.example.com/v1".into());
        assert_eq!(config.effective_api_key("provider-key"), "provider-key");
        assert_eq!(
            config.effective_base_url("https://provider.example.com"),
            "https://api.example.com/v1"
        );
    }

    #[test]
    fn parse_result_supports_object_and_array() {
        let single = parse_usage_result(serde_json::json!({"remaining": 1.5, "unit": "USD"}));
        assert!(single.success);
        assert_eq!(single.data.len(), 1);
        assert_eq!(single.data[0].remaining, Some(1.5));

        let multi =
            parse_usage_result(serde_json::json!([{"remaining": 1.0}, {"planName": "Pro"}]));
        assert_eq!(multi.data.len(), 2);
        assert_eq!(multi.data[1].plan_name.as_deref(), Some("Pro"));
    }

    #[test]
    fn preset_templates_are_non_empty_js() {
        for template in [TEMPLATE_CUSTOM, TEMPLATE_GENERAL, TEMPLATE_NEW_API] {
            let code = preset_template(template);
            assert!(code.contains("request:"), "{template}");
            assert!(code.contains("extractor"), "{template}");
        }
    }
}

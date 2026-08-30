//! 用量查询脚本执行引擎(移植自 cc-switch 的 usage_script 设计)。
//!
//! 脚本是一段返回 `({ request: {...}, extractor: function(response) {...} })`
//! 的 JavaScript。执行流程:
//! 1. 替换 `{{apiKey}} / {{baseUrl}} / {{accessToken}} / {{userId}}` 模板变量;
//! 2. 在受限 QuickJS 运行时(内存/栈/时间片上限)里 eval 出 `request` 配置;
//! 3. 发送 HTTP 请求(超时受配置约束,钳制在 2~30 秒);
//! 4. 再次 eval 取出 `extractor` 并把响应 JSON 传入,得到用量字段;
//! 5. 校验返回结构(对象或对象数组,字段类型受控)。
//!
//! 安全约束:非自定义模板强制 HTTPS(回环地址除外)且请求与 base_url 同源;
//! 脚本来自不可信输入(导入/同步),因此限制 CPU/内存防 DoS。

use std::collections::HashMap;

use rquickjs::{Context, Function, Runtime};
use serde_json::Value;
use url::{Host, Url};

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct UsageQueryError(pub String);

type Result<T> = std::result::Result<T, UsageQueryError>;

fn err(message: impl Into<String>) -> UsageQueryError {
    UsageQueryError(message.into())
}

/// 用量脚本允许的最长执行时间(秒)。
const USAGE_SCRIPT_TIMEOUT_SECS: u64 = 5;
/// 16 MiB 对仅构造 request 配置 / extractor 的脚本已经足够。
const USAGE_SCRIPT_MEMORY_LIMIT_BYTES: usize = 16 * 1024 * 1024;

/// 执行用量查询脚本,返回 extractor 的原始 JSON 结果(已校验)。
#[allow(clippy::too_many_arguments)]
pub async fn execute_usage_script(
    script_code: &str,
    api_key: &str,
    base_url: &str,
    timeout_secs: u64,
    access_token: Option<&str>,
    user_id: Option<&str>,
    template_type: Option<&str>,
) -> Result<Value> {
    let is_custom_template = template_type.is_some_and(|t| t == domain::TEMPLATE_CUSTOM);

    // 1. 替换模板变量,避免敏感信息进入脚本源码之外的路径
    let script_with_vars =
        build_script_with_vars(script_code, api_key, base_url, access_token, user_id);

    // 2. base_url 安全性(自定义模板可能完全不使用该变量,跳过)
    if !base_url.is_empty() && !is_custom_template {
        validate_base_url(base_url)?;
    }

    // 3. 提取 request 配置(Runtime/Context 在 await 前释放)
    let request_json = {
        let runtime = create_script_runtime()?;
        let context =
            Context::full(&runtime).map_err(|e| err(format!("创建 JS 上下文失败: {e}")))?;

        context.with(|ctx| {
            let config: rquickjs::Object = ctx
                .eval(script_with_vars.clone())
                .map_err(|e| err(format!("解析配置失败: {e}")))?;
            let request: rquickjs::Object = config
                .get("request")
                .map_err(|e| err(format!("缺少 request 配置: {e}")))?;
            let request_json: String = ctx
                .json_stringify(request)
                .map_err(|e| err(format!("序列化 request 失败: {e}")))?
                .ok_or_else(|| err("序列化返回 None"))?
                .get()
                .map_err(|e| err(format!("获取字符串失败: {e}")))?;
            Ok::<_, UsageQueryError>(request_json)
        })?
    };

    let request: RequestConfig = serde_json::from_str(&request_json)
        .map_err(|e| err(format!("request 配置格式错误: {e}")))?;

    // 4. 请求 URL 校验(HTTPS 强制 + 同源)
    validate_request_url(&request.url, base_url, is_custom_template)?;

    // 5. 发送 HTTP 请求
    let response_data = send_http_request(&request, timeout_secs).await?;

    // 6. 执行 extractor
    let result: Value = {
        let runtime = create_script_runtime()?;
        let context =
            Context::full(&runtime).map_err(|e| err(format!("创建 JS 上下文失败: {e}")))?;

        context.with(|ctx| {
            let config: rquickjs::Object = ctx
                .eval(script_with_vars.clone())
                .map_err(|e| err(format!("重新解析配置失败: {e}")))?;
            let extractor: Function = config
                .get("extractor")
                .map_err(|e| err(format!("缺少 extractor 函数: {e}")))?;
            let response_js: rquickjs::Value = ctx
                .json_parse(response_data.as_str())
                .map_err(|e| err(format!("解析响应 JSON 失败: {e}")))?;
            let result_js: rquickjs::Value = extractor
                .call((response_js,))
                .map_err(|e| err(format!("执行 extractor 失败: {e}")))?;
            let result_json: String = ctx
                .json_stringify(result_js)
                .map_err(|e| err(format!("序列化结果失败: {e}")))?
                .ok_or_else(|| err("序列化返回 None"))?
                .get()
                .map_err(|e| err(format!("获取字符串失败: {e}")))?;
            serde_json::from_str(&result_json).map_err(|e| err(format!("JSON 解析失败: {e}")))
        })?
    };

    // 7. 返回值结构校验
    validate_result(&result)?;

    Ok(result)
}

#[derive(Debug, serde::Deserialize)]
struct RequestConfig {
    url: String,
    method: String,
    #[serde(default)]
    headers: HashMap<String, String>,
    #[serde(default)]
    body: Option<String>,
}

async fn send_http_request(config: &RequestConfig, timeout_secs: u64) -> Result<String> {
    let client = reqwest::Client::builder().build().unwrap_or_default();
    // 钳制超时范围,防止异常配置导致长时间阻塞
    let request_timeout = std::time::Duration::from_secs(timeout_secs.clamp(2, 30));

    let method: reqwest::Method = config
        .method
        .parse()
        .map_err(|_| err(format!("不支持的 HTTP 方法: {}", config.method)))?;

    let mut req = client.request(method, &config.url).timeout(request_timeout);
    for (name, value) in &config.headers {
        req = req.header(name, value);
    }
    if let Some(body) = &config.body {
        req = req.body(body.clone());
    }

    let resp = req
        .send()
        .await
        .map_err(|e| err(format!("请求失败: {e}")))?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| err(format!("读取响应失败: {e}")))?;

    if !status.is_success() {
        let preview = if text.len() > 200 {
            let mut safe_cut = 200usize;
            while !text.is_char_boundary(safe_cut) {
                safe_cut = safe_cut.saturating_sub(1);
            }
            format!("{}...", &text[..safe_cut])
        } else {
            text.clone()
        };
        return Err(err(format!("HTTP {status} : {preview}")));
    }

    Ok(text)
}

/// 创建受控 QuickJS 运行时:内存/栈上限 + 执行时间中断器。
fn create_script_runtime() -> Result<Runtime> {
    let runtime = Runtime::new().map_err(|e| err(format!("创建 JS 运行时失败: {e}")))?;
    runtime.set_memory_limit(USAGE_SCRIPT_MEMORY_LIMIT_BYTES);
    runtime.set_max_stack_size(256 * 1024);
    let deadline = std::time::Instant::now()
        .checked_add(std::time::Duration::from_secs(USAGE_SCRIPT_TIMEOUT_SECS));
    if let Some(deadline) = deadline {
        runtime.set_interrupt_handler(Some(Box::new(move || std::time::Instant::now() > deadline)));
    }
    Ok(runtime)
}

/// 验证脚本返回值(支持单对象或数组)
fn validate_result(result: &Value) -> Result<()> {
    if let Some(arr) = result.as_array() {
        if arr.is_empty() {
            return Err(err("脚本返回的数组不能为空"));
        }
        for (idx, item) in arr.iter().enumerate() {
            validate_single_usage(item)
                .map_err(|e| err(format!("数组索引[{idx}]验证失败: {e}")))?;
        }
        return Ok(());
    }
    validate_single_usage(result)
}

/// 验证单个用量数据对象:所有字段可选,仅做类型检查
fn validate_single_usage(result: &Value) -> Result<()> {
    let obj = result
        .as_object()
        .ok_or_else(|| err("脚本必须返回对象或对象数组"))?;

    let type_errors: [(&str, bool, &str); 7] = [
        (
            "isValid",
            !result["isValid"].is_boolean(),
            "isValid 必须是布尔值或 null",
        ),
        (
            "invalidMessage",
            !result["invalidMessage"].is_string(),
            "invalidMessage 必须是字符串或 null",
        ),
        (
            "remaining",
            !result["remaining"].is_number(),
            "remaining 必须是数字或 null",
        ),
        (
            "unit",
            !result["unit"].is_string(),
            "unit 必须是字符串或 null",
        ),
        (
            "total",
            !result["total"].is_number(),
            "total 必须是数字或 null",
        ),
        (
            "used",
            !result["used"].is_number(),
            "used 必须是数字或 null",
        ),
        (
            "planName",
            !result["planName"].is_string(),
            "planName 必须是字符串或 null",
        ),
    ];
    for (key, present_and_bad, message) in type_errors {
        if obj.contains_key(key) && !result[key].is_null() && present_and_bad {
            return Err(err(message));
        }
    }
    if obj.contains_key("extra") && !result["extra"].is_null() && !result["extra"].is_string() {
        return Err(err("extra 必须是字符串或 null"));
    }

    Ok(())
}

/// 构建替换变量后的脚本
fn build_script_with_vars(
    script_code: &str,
    api_key: &str,
    base_url: &str,
    access_token: Option<&str>,
    user_id: Option<&str>,
) -> String {
    let mut replaced = script_code
        .replace("{{apiKey}}", api_key)
        .replace("{{baseUrl}}", base_url);
    if let Some(token) = access_token {
        replaced = replaced.replace("{{accessToken}}", token);
    }
    if let Some(uid) = user_id {
        replaced = replaced.replace("{{userId}}", uid);
    }
    replaced
}

/// 验证 base_url 的基本安全性(HTTPS 强制,回环地址除外)
fn validate_base_url(base_url: &str) -> Result<()> {
    let parsed_url = Url::parse(base_url).map_err(|e| err(format!("无效的 base_url: {e}")))?;
    let is_loopback = is_loopback_host(&parsed_url);

    if parsed_url.scheme() != "https" && !is_loopback {
        return Err(err("base_url 必须使用 HTTPS 协议(localhost 除外)"));
    }

    let hostname = parsed_url
        .host_str()
        .ok_or_else(|| err("base_url 必须包含有效的主机名"))?;
    if hostname.is_empty() {
        return Err(err("base_url 主机名不能为空"));
    }

    Ok(())
}

/// 验证请求 URL 是否安全(HTTPS 强制 + 与 base_url 同源;自定义模板放开)
fn validate_request_url(request_url: &str, base_url: &str, is_custom_template: bool) -> Result<()> {
    let parsed_request =
        Url::parse(request_url).map_err(|e| err(format!("无效的请求 URL: {e}")))?;
    let is_request_loopback = is_loopback_host(&parsed_request);

    if !is_custom_template && parsed_request.scheme() != "https" && !is_request_loopback {
        return Err(err("请求 URL 必须使用 HTTPS 协议(localhost 除外)"));
    }

    if !base_url.is_empty() && !is_custom_template {
        let parsed_base = Url::parse(base_url).map_err(|e| err(format!("无效的 base_url: {e}")))?;

        if parsed_request.host_str() != parsed_base.host_str() {
            return Err(err(format!(
                "请求域名 {} 与 base_url 域名 {} 不匹配(必须是同源请求)",
                parsed_request.host_str().unwrap_or("unknown"),
                parsed_base.host_str().unwrap_or("unknown")
            )));
        }

        match (
            parsed_request.port_or_known_default(),
            parsed_base.port_or_known_default(),
        ) {
            (Some(request_port), Some(base_port)) if request_port == base_port => {}
            (Some(request_port), Some(base_port)) => {
                return Err(err(format!(
                    "请求端口 {request_port} 必须与 base_url 端口 {base_port} 匹配"
                )));
            }
            _ => return Err(err("无法确定端口号")),
        }
    }

    Ok(())
}

/// 判断 URL 是否指向本机(localhost / loopback)
fn is_loopback_host(url: &Url) -> bool {
    match url.host() {
        Some(Host::Domain(domain)) => domain.eq_ignore_ascii_case("localhost"),
        Some(Host::Ipv4(ip)) => ip.is_loopback(),
        Some(Host::Ipv6(ip)) => ip.is_loopback(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn https_bypass_prevention() {
        assert!(validate_base_url("http://127.0.0.1.evil.com/api").is_err());
    }

    #[test]
    fn custom_template_allows_http_lan_request() {
        assert!(validate_request_url(
            "http://10.0.0.8:18344/user/balance",
            "http://10.0.0.8:8090/anthropic",
            true,
        )
        .is_ok());
    }

    #[test]
    fn port_comparison_handles_default_ports() {
        let cases = [
            (
                "https://api.example.com",
                "https://api.example.com/v1/test",
                true,
            ),
            (
                "https://api.example.com",
                "https://api.example.com:443/v1/test",
                true,
            ),
            (
                "https://api.example.com",
                "https://api.example.com:8443/v1/test",
                false,
            ),
        ];
        for (base_url, request_url, should_match) in cases {
            let result = validate_request_url(request_url, base_url, false);
            assert_eq!(result.is_ok(), should_match, "{base_url} -> {request_url}");
        }
    }

    #[test]
    fn infinite_loop_script_is_interrupted() {
        let script = r#"
            (function(){
                while (true) { Math.sqrt(Math.random()); }
            })();
            ({ request: { url: "https://example.com", method: "GET" } })
        "#;
        let start = std::time::Instant::now();
        let result = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("tokio runtime for test")
            .block_on(execute_usage_script(
                script,
                "sk-test",
                "https://api.example.com",
                30,
                None,
                None,
                None,
            ));
        assert!(result.is_err(), "infinite loop script must be rejected");
        assert!(
            start.elapsed() < std::time::Duration::from_secs(15),
            "interruption took too long: {:?}",
            start.elapsed()
        );
    }

    #[tokio::test]
    async fn general_template_roundtrip_against_mock_upstream() {
        // mock 上游:GET /user/balance 返回余额 JSON
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };
                tokio::spawn(async move {
                    use tokio::io::{AsyncReadExt, AsyncWriteExt};
                    let mut buf = [0u8; 2048];
                    let _ = socket.read(&mut buf).await;
                    let body = br#"{"is_active": true, "balance": 12.5}"#;
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        String::from_utf8_lossy(body)
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                });
            }
        });

        let script = domain::preset_template(domain::TEMPLATE_GENERAL)
            .replace("{{baseUrl}}", &format!("http://{addr}"))
            .replace("{{apiKey}}", "sk-mock-key-12345");
        // 通用模板强制同源 + HTTPS,回环地址允许 HTTP
        let result = execute_usage_script(&script, "sk-mock-key-12345", "", 10, None, None, None)
            .await
            .expect("query should succeed");
        assert_eq!(result["remaining"], 12.5);
        assert_eq!(result["unit"], "USD");
    }

    #[test]
    fn result_type_validation() {
        assert!(validate_single_usage(&serde_json::json!({"remaining": "12"})).is_err());
        assert!(validate_single_usage(&serde_json::json!({"remaining": 12.0})).is_ok());
        assert!(validate_single_usage(&serde_json::json!([])).is_err());
    }
}

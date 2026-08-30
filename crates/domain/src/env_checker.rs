use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolInstallation {
    pub path: String,
    pub version: Option<String>,
    pub runnable: bool,
    pub error: Option<String>,
    pub source: String,
    pub is_path_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolEnvironmentStatus {
    pub id: String,
    pub name: String,
    pub icon_kind: String,
    pub platform: String,
    pub current_version: Option<String>,
    pub latest_version: Option<String>,
    pub is_installed: bool,
    pub is_upgradable: bool,
    pub installed_but_broken: bool,
    pub error: Option<String>,
    pub installations: Vec<ToolInstallation>,
    pub has_conflicts: bool,
}

/// Parse a semver string like "2.1.238" or "0.147.0-alpha.6.6" into ([major, minor, patch], prerelease)
pub fn parse_semver(v: &str) -> Option<([u64; 3], Vec<String>)> {
    let core_and_pre = v.trim().split('+').next().unwrap_or("");
    let (core, pre) = match core_and_pre.split_once('-') {
        Some((c, p)) => (c, Some(p)),
        None => (core_and_pre, None),
    };
    let mut parts = core.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next()?.parse::<u64>().ok()?;
    let patch = parts.next()?.parse::<u64>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    let pre_segments = pre
        .map(|p| p.split('.').map(|s| s.to_string()).collect())
        .unwrap_or_default();
    Some(([major, minor, patch], pre_segments))
}

/// Compare two semver strings according to semver 2.0 specifications
pub fn compare_semver(a: &str, b: &str) -> Option<std::cmp::Ordering> {
    use std::cmp::Ordering;
    let (ac, ap) = parse_semver(a)?;
    let (bc, bp) = parse_semver(b)?;
    for i in 0..3 {
        match ac[i].cmp(&bc[i]) {
            Ordering::Equal => continue,
            other => return Some(other),
        }
    }
    match (ap.is_empty(), bp.is_empty()) {
        (true, true) => return Some(Ordering::Equal),
        (true, false) => return Some(Ordering::Greater),
        (false, true) => return Some(Ordering::Less),
        (false, false) => {}
    }
    for (x, y) in ap.iter().zip(bp.iter()) {
        let ord = match (x.parse::<u64>(), y.parse::<u64>()) {
            (Ok(xv), Ok(yv)) => xv.cmp(&yv),
            (Ok(_), Err(_)) => Ordering::Less,
            (Err(_), Ok(_)) => Ordering::Greater,
            (Err(_), Err(_)) => x.as_str().cmp(y.as_str()),
        };
        if ord != Ordering::Equal {
            return Some(ord);
        }
    }
    Some(ap.len().cmp(&bp.len()))
}

/// Determine if current version is strictly older than latest version
pub fn is_version_outdated(current: Option<&str>, latest: Option<&str>) -> bool {
    let (Some(curr), Some(lat)) = (current, latest) else {
        return false;
    };
    compare_semver(curr, lat) == Some(std::cmp::Ordering::Less)
}

/// Extract clean semver from raw command output (e.g. "codex-cli 0.147.0-alpha.6.6" -> "0.147.0-alpha.6.6")
pub fn extract_version(raw: &str) -> String {
    let trimmed = raw.trim();
    for line in trimmed.lines() {
        for word in line.split_whitespace() {
            let clean = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '.' && c != '-');
            if parse_semver(clean).is_some() {
                return clean.to_string();
            }
            if let Some(stripped) = clean.strip_prefix('v') {
                if parse_semver(stripped).is_some() {
                    return stripped.to_string();
                }
            }
        }
    }
    trimmed.to_string()
}

/// Infer source tag from binary path (nvm, homebrew, volta, pnpm, etc.)
pub fn infer_install_source(path: &Path) -> &'static str {
    let s = path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    if s.contains("/.nvm/") {
        "nvm"
    } else if s.contains("/homebrew/") || s.contains("/cellar/") {
        "homebrew"
    } else if s.contains("/.volta/") || s.contains("/volta/") {
        "volta"
    } else if s.contains("fnm_multishells") {
        "fnm"
    } else if s.contains("/mise/") {
        "mise"
    } else if s.contains("/.bun/") {
        "bun"
    } else if s.contains("/pnpm/") {
        "pnpm"
    } else if s.contains("/scoop/") {
        "scoop"
    } else if s.contains("/library/python")
        || s.contains("/scripts/")
        || s.contains("/site-packages/")
    {
        "pip"
    } else if s.contains("/.opencode/") {
        "opencode"
    } else if s.contains("/.grok/") {
        "grok"
    } else {
        "system"
    }
}

fn push_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.contains(&path) {
        paths.push(path);
    }
}

/// Windows: 展开 `%VAR%` 环境变量引用。注册表 `Path` 是 `REG_EXPAND_SZ`,
/// winreg 读出的字符串不会自动展开; Winlogon 启动的进程即使丢失用户 PATH,
/// `%USERPROFILE%` / `%LOCALAPPDATA%` / `%SystemRoot%` 等变量仍然存在,
/// 因此逐个用 `std::env::var` 展开是安全的。未定义的变量原样保留。
#[cfg(target_os = "windows")]
fn expand_env_chars(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) => {
                let var = &after[..end];
                match std::env::var(var) {
                    Ok(value) => out.push_str(&value),
                    Err(_) => out.push_str(&format!("%{var}%")),
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Windows: 合并注册表(HKLM + HKCU)与进程 PATH, 去重保序。
/// 进程 PATH 排最前(继承到的仍生效), 其后用户、机器——与"新开终端看到的 PATH"
/// 的解析顺序一致, 修复应用内自更新重启后用户 PATH 丢失导致「未安装」的问题。
#[cfg(target_os = "windows")]
pub fn effective_path_paths() -> Vec<PathBuf> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;

    let process = std::env::var("PATH").unwrap_or_default();
    let user = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey("Environment")
        .and_then(|key| key.get_value::<String, _>("Path"))
        .map(|raw| expand_env_chars(&raw))
        .unwrap_or_default();
    let machine = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey("SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment")
        .and_then(|key| key.get_value::<String, _>("Path"))
        .map(|raw| expand_env_chars(&raw))
        .unwrap_or_default();

    let mut out = Vec::new();
    for segment in [process, user, machine] {
        for part in segment.split(';') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            push_unique_path(&mut out, PathBuf::from(part));
        }
    }
    out
}

/// 非 Windows: 直接使用进程 PATH
#[cfg(not(target_os = "windows"))]
pub fn effective_path_paths() -> Vec<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .filter(|p| !p.as_os_str().is_empty())
        .collect()
}

/// Build search paths for locating candidate binaries across the system.
///
/// PATH(Windows 下为注册表合并后的有效 PATH)条目排在最前——终端里 `tool`
/// 解析到哪个二进制, 检测就优先报告哪个, 避免 `~/.local/bin` 里的旧 npm shim
/// 遮蔽新装版本; 版本管理器与硬编码回退目录在其后作为兜底。
pub fn build_tool_search_paths(tool: &str) -> Vec<PathBuf> {
    let mut search_paths = Vec::new();
    let home = dirs::home_dir().unwrap_or_default();

    // 1) 有效 PATH 优先
    for part in effective_path_paths() {
        push_unique_path(&mut search_paths, part);
    }

    if tool == "grok" && !home.as_os_str().is_empty() {
        push_unique_path(&mut search_paths, home.join(".grok/bin"));
    }
    if tool == "opencode" && !home.as_os_str().is_empty() {
        push_unique_path(&mut search_paths, home.join(".opencode/bin"));
    }

    #[cfg(target_os = "windows")]
    {
        // 独立安装器目录(winget / 官方安装包, 可能从未写入 PATH)
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            let local_app_data = PathBuf::from(local_app_data);
            if tool == "codex" {
                push_unique_path(
                    &mut search_paths,
                    local_app_data
                        .join("Programs")
                        .join("OpenAI")
                        .join("Codex")
                        .join("bin"),
                );
            }
            if tool == "claude" {
                push_unique_path(
                    &mut search_paths,
                    local_app_data.join("Programs").join("claude"),
                );
            }
        }
        // npm 全局目录(自定义 prefix 场景)
        if let Some(app_data) = std::env::var_os("APPDATA") {
            push_unique_path(&mut search_paths, PathBuf::from(app_data).join("npm"));
        }
    }

    if !home.as_os_str().is_empty() {
        push_unique_path(&mut search_paths, home.join(".local/bin"));
        push_unique_path(&mut search_paths, home.join(".npm-global/bin"));
        push_unique_path(&mut search_paths, home.join("n/bin"));
        push_unique_path(&mut search_paths, home.join(".volta/bin"));
        push_unique_path(&mut search_paths, home.join("bin"));
    }

    #[cfg(target_os = "macos")]
    {
        push_unique_path(&mut search_paths, PathBuf::from("/opt/homebrew/bin"));
        push_unique_path(&mut search_paths, PathBuf::from("/usr/local/bin"));
        push_unique_path(&mut search_paths, PathBuf::from("/usr/bin"));
        push_unique_path(&mut search_paths, PathBuf::from("/bin"));
    }

    #[cfg(target_os = "linux")]
    {
        push_unique_path(&mut search_paths, PathBuf::from("/usr/local/bin"));
        push_unique_path(&mut search_paths, PathBuf::from("/usr/bin"));
        push_unique_path(&mut search_paths, PathBuf::from("/bin"));
    }

    if !home.as_os_str().is_empty() {
        let nvm_base = home.join(".nvm/versions/node");
        if nvm_base.exists() {
            if let Ok(entries) = std::fs::read_dir(&nvm_base) {
                for entry in entries.flatten() {
                    let bin_path = entry.path().join("bin");
                    if bin_path.exists() {
                        push_unique_path(&mut search_paths, bin_path);
                    }
                }
            }
        }
        let fnm_base = home.join(".local/state/fnm_multishells");
        if fnm_base.exists() {
            if let Ok(entries) = std::fs::read_dir(&fnm_base) {
                for entry in entries.flatten() {
                    let bin_path = entry.path().join("bin");
                    if bin_path.exists() {
                        push_unique_path(&mut search_paths, bin_path);
                    }
                }
            }
        }
        let mise_base = home.join(".local/share/mise/installs/node");
        if mise_base.exists() {
            if let Ok(entries) = std::fs::read_dir(&mise_base) {
                for entry in entries.flatten() {
                    let bin_path = entry.path().join("bin");
                    if bin_path.exists() {
                        push_unique_path(&mut search_paths, bin_path);
                    }
                }
            }
        }
    }

    search_paths
}

/// Resolve the default executable in user's login shell PATH
pub fn resolve_path_default(tool: &str) -> Option<PathBuf> {
    #[cfg(not(target_os = "windows"))]
    {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "zsh".to_string());
        let output = Command::new(&shell)
            .arg("-lic")
            .arg(format!("command -v {}", tool))
            .output()
            .ok()?;
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines().map(str::trim) {
                if line.starts_with('/') {
                    let p = PathBuf::from(line);
                    return std::fs::canonicalize(&p).ok().or(Some(p));
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        // 用合并注册表后的有效 PATH 执行 where, 并跳过 Windows 应用执行别名
        // (Microsoft\WindowsApps 下的重解析点只会拉起商店/协议处理器)
        let effective = effective_path_paths();
        let mut command = Command::new("cmd");
        command.args(["/C", &format!("where {tool}")]);
        let joined = effective
            .iter()
            .filter(|dir| {
                !dir.to_string_lossy()
                    .to_ascii_lowercase()
                    .contains("microsoft\\windowsapps")
            })
            .map(|dir| dir.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(";");
        command.env("PATH", &joined);
        let output = command.output().ok()?;
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines().map(str::trim) {
                if line.is_empty() || line.to_ascii_lowercase().contains("microsoft\\windowsapps") {
                    continue;
                }
                let p = PathBuf::from(line);
                return std::fs::canonicalize(&p).ok().or(Some(p));
            }
        }
    }

    None
}

/// Query package registry information for a tool
pub fn package_info_for(tool_id: &str) -> (&'static str, &'static str) {
    match tool_id {
        "claude" => ("@anthropic-ai/claude-code", "npm"),
        "codex" => ("@openai/codex", "npm"),
        "gemini" => ("@google/gemini-cli", "npm"),
        "grok" => ("@xai-official/grok", "npm"),
        "opencode" => ("opencode-ai", "npm"),
        "pi" => ("@earendil-works/pi-coding-agent", "npm"),
        "openclaw" => ("openclaw", "npm"),
        "hermes" => ("hermes-agent", "pypi"),
        _ => ("", ""),
    }
}

/// Fetch latest remote version from npm registry or PyPI
pub fn fetch_remote_latest_version(tool_id: &str) -> Option<String> {
    let (pkg, registry_type) = package_info_for(tool_id);
    if pkg.is_empty() {
        return None;
    }
    match registry_type {
        "npm" => {
            let url = format!("https://registry.npmjs.org/{}", pkg);
            let resp = ureq::get(&url)
                .timeout(std::time::Duration::from_secs(4))
                .call()
                .ok()?;
            let val: serde_json::Value = resp.into_json().ok()?;
            let dist_tags = val.get("dist-tags")?.as_object()?;
            dist_tags.get("latest")?.as_str().map(|s| s.to_string())
        }
        "pypi" => {
            let url = format!("https://pypi.org/pypi/{}/json", pkg);
            let resp = ureq::get(&url)
                .timeout(std::time::Duration::from_secs(4))
                .call()
                .ok()?;
            let val: serde_json::Value = resp.into_json().ok()?;
            val.get("info")?
                .get("version")?
                .as_str()
                .map(|s| s.to_string())
        }
        _ => None,
    }
}

/// Full inspection of a tool in local system environment
pub fn inspect_tool_environment(
    tool_id: &str,
    tool_display_name: &str,
    fetch_remote: bool,
) -> ToolEnvironmentStatus {
    let mut search_paths = build_tool_search_paths(tool_id);
    let path_default = resolve_path_default(tool_id);
    if let Some(ref def_path) = path_default {
        if let Some(parent) = def_path.parent() {
            push_unique_path(&mut search_paths, parent.to_path_buf());
        }
    }
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut installations: Vec<ToolInstallation> = Vec::new();

    for dir in &search_paths {
        let tool_path = dir.join(tool_id);
        if !tool_path.exists() {
            continue;
        }

        let real = std::fs::canonicalize(&tool_path).unwrap_or_else(|_| tool_path.clone());
        if !seen.insert(real.clone()) {
            continue;
        }

        let output = Command::new(&tool_path).arg("--version").output();

        let (version, runnable, error) = match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                let raw = if stdout.is_empty() { stderr } else { stdout };
                (Some(extract_version(&raw)), true, None)
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let detail = if stderr.is_empty() { stdout } else { stderr };
                let err_msg = detail.lines().rev().take(3).collect::<Vec<_>>().join(" ");
                (
                    None,
                    false,
                    Some(if err_msg.is_empty() {
                        "执行返回非零状态".into()
                    } else {
                        err_msg
                    }),
                )
            }
            Err(e) => (None, false, Some(e.to_string())),
        };

        let is_path_default = path_default.as_ref() == Some(&real);
        let path_str = tool_path.display().to_string();
        let source = infer_install_source(&tool_path).to_string();

        installations.push(ToolInstallation {
            path: path_str,
            version,
            runnable,
            error,
            source,
            is_path_default,
        });
    }

    // Sort: default path first
    installations.sort_by_key(|i| std::cmp::Reverse(i.is_path_default));

    let default_inst = installations
        .iter()
        .find(|i| i.is_path_default)
        .or_else(|| installations.first());
    let current_version = default_inst.and_then(|i| i.version.clone());
    let is_installed = !installations.is_empty();
    let installed_but_broken =
        is_installed && current_version.is_none() && installations.iter().any(|i| !i.runnable);
    let error = default_inst.and_then(|i| i.error.clone());

    let latest_version = if fetch_remote {
        fetch_remote_latest_version(tool_id)
    } else {
        None
    };

    let is_upgradable = is_version_outdated(current_version.as_deref(), latest_version.as_deref());

    // Conflict detection: ≥ 2 distinct installations
    let has_conflicts = installations.len() >= 2;

    let platform = if cfg!(target_os = "macos") {
        "macOS".to_string()
    } else if cfg!(target_os = "windows") {
        "Windows".to_string()
    } else {
        "Linux".to_string()
    };

    ToolEnvironmentStatus {
        id: tool_id.to_string(),
        name: tool_display_name.to_string(),
        icon_kind: tool_id.to_string(),
        platform,
        current_version,
        latest_version,
        is_installed,
        is_upgradable,
        installed_but_broken,
        error,
        installations,
        has_conflicts,
    }
}

/// Inspect all supported CLI tools
pub fn inspect_all_tools(fetch_remote: bool) -> Vec<ToolEnvironmentStatus> {
    let tools = [
        ("claude", "Claude Code"),
        ("codex", "Codex"),
        ("gemini", "Gemini CLI"),
        ("grok", "Grok Build"),
        ("opencode", "OpenCode"),
        ("pi", "Pi"),
        ("openclaw", "OpenClaw"),
        ("hermes", "Hermes"),
    ];

    tools
        .iter()
        .map(|(id, name)| inspect_tool_environment(id, name, fetch_remote))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_and_compare_semver() {
        assert_eq!(
            compare_semver("2.1.238", "2.1.241"),
            Some(std::cmp::Ordering::Less)
        );
        assert_eq!(
            compare_semver("0.147.0-alpha.6.6", "0.149.0"),
            Some(std::cmp::Ordering::Less)
        );
        assert_eq!(
            compare_semver("1.0.5", "1.0.5"),
            Some(std::cmp::Ordering::Equal)
        );
        assert_eq!(
            compare_semver("2.0.0", "1.9.9"),
            Some(std::cmp::Ordering::Greater)
        );
        assert!(is_version_outdated(Some("2.1.238"), Some("2.1.241")));
        assert!(!is_version_outdated(Some("2.1.241"), Some("2.1.241")));
        assert!(!is_version_outdated(Some("2.1.242"), Some("2.1.241")));
    }

    #[test]
    fn test_extract_version() {
        assert_eq!(extract_version("2.1.238 (Claude Code)"), "2.1.238");
        assert_eq!(
            extract_version("codex-cli 0.147.0-alpha.6.6"),
            "0.147.0-alpha.6.6"
        );
        assert_eq!(extract_version("grok 1.0.5 (5115b46bc909)"), "1.0.5");
        assert_eq!(extract_version("v1.17.9"), "1.17.9");
    }

    #[test]
    fn test_infer_install_source() {
        assert_eq!(
            infer_install_source(Path::new(
                "/Users/wayne/.nvm/versions/node/v24.14.0/bin/claude"
            )),
            "nvm"
        );
        assert_eq!(
            infer_install_source(Path::new("/opt/homebrew/bin/codex")),
            "homebrew"
        );
        assert_eq!(
            infer_install_source(Path::new("/Users/wayne/.volta/bin/grok")),
            "volta"
        );
        assert_eq!(
            infer_install_source(Path::new("/Users/wayne/.opencode/bin/opencode")),
            "opencode"
        );
        assert_eq!(
            infer_install_source(Path::new("/usr/local/bin/pi")),
            "system"
        );
    }
}

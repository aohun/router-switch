use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::env_checker::{compare_semver, extract_version};

/// Information about an application release fetched from a remote source (e.g. GitHub Releases).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppRelease {
    pub version: String,
    pub title: String,
    pub body: String,
    pub release_notes_zh: Vec<String>,
    pub release_notes_en: Vec<String>,
    pub download_url: String,
    pub html_url: String,
    pub published_at: Option<String>,
    pub is_prerelease: bool,
}

#[derive(Debug, Deserialize)]
struct GithubReleaseAsset {
    name: Option<String>,
    browser_download_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: Option<String>,
    name: Option<String>,
    body: Option<String>,
    html_url: Option<String>,
    published_at: Option<String>,
    prerelease: Option<bool>,
    assets: Option<Vec<GithubReleaseAsset>>,
}

/// Parse release notes into structured sections (Chinese & English bullet points).
pub fn parse_release_notes(raw_body: &str) -> (Vec<String>, Vec<String>) {
    let mut zh_notes = Vec::new();
    let mut en_notes = Vec::new();

    let mut current_section: Option<&str> = None;

    for line in raw_body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with('#')
            || lower.contains("新变化")
            || lower.contains("更新日志")
            || lower.contains("changelog")
            || lower.contains("what's new")
            || lower.contains("whats new")
        {
            if trimmed.contains("新变化")
                || trimmed.contains("更新说明")
                || trimmed.contains("更新日志")
                || trimmed.contains("中文")
            {
                current_section = Some("zh");
                continue;
            } else if lower.contains("what's new")
                || lower.contains("whats new")
                || lower.contains("english")
                || lower.contains("changelog")
            {
                current_section = Some("en");
                continue;
            }
        }

        // Clean markdown bullet points (-, *, •, \d+.)
        let content = clean_bullet_line(trimmed);
        if content.is_empty() {
            continue;
        }

        match current_section {
            Some("zh") => zh_notes.push(content),
            Some("en") => en_notes.push(content),
            _ => {
                // If contains Chinese characters, categorize as zh, otherwise en
                if content
                    .chars()
                    .any(|c| ('\u{4E00}'..='\u{9FFF}').contains(&c))
                {
                    zh_notes.push(content);
                } else {
                    en_notes.push(content);
                }
            }
        }
    }

    (zh_notes, en_notes)
}

fn clean_bullet_line(line: &str) -> String {
    let s = line.trim();
    let stripped = if let Some(rest) = s.strip_prefix("- ") {
        rest
    } else if let Some(rest) = s.strip_prefix("* ") {
        rest
    } else if let Some(rest) = s.strip_prefix("• ") {
        rest
    } else if let Some(rest) = s.strip_prefix("+ ") {
        rest
    } else {
        // Check for numeric lists like "1. "
        if let Some(pos) = s.find(". ") {
            let prefix = &s[..pos];
            if prefix.chars().all(|c| c.is_ascii_digit()) {
                &s[pos + 2..]
            } else {
                s
            }
        } else {
            s
        }
    };
    stripped.trim().to_string()
}

/// Check for remote application updates from a GitHub repository (e.g. "aohun/router-switch").
pub fn check_app_update(repo: &str, current_version: &str) -> Result<Option<AppRelease>, String> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let resp = ureq::get(&url)
        .set("User-Agent", "router-switch-app")
        .set("Accept", "application/vnd.github.v3+json")
        .timeout(Duration::from_secs(8))
        .call()
        .map_err(|e| format!("网络请求失败: {e}"))?;

    let release: GithubRelease = resp
        .into_json()
        .map_err(|e| format!("解析发布信息失败: {e}"))?;

    let tag = release.tag_name.as_deref().unwrap_or("");
    let latest_ver = extract_version(tag);
    let curr_ver = extract_version(current_version);

    // If latest version is strictly newer than current version
    if compare_semver(&curr_ver, &latest_ver) == Some(std::cmp::Ordering::Less) {
        let body = release.body.unwrap_or_default();
        let (zh, en) = parse_release_notes(&body);

        let html_url = release
            .html_url
            .unwrap_or_else(|| format!("https://github.com/{repo}/releases"));

        // Match platform specific download asset if available
        let download_url = release
            .assets
            .as_ref()
            .and_then(|assets| {
                #[cfg(target_os = "macos")]
                {
                    assets
                        .iter()
                        .find(|a| {
                            a.name
                                .as_ref()
                                .map(|n| {
                                    n.ends_with(".dmg")
                                        || n.ends_with(".pkg")
                                        || n.ends_with(".zip")
                                })
                                .unwrap_or(false)
                        })
                        .and_then(|a| a.browser_download_url.clone())
                }
                #[cfg(target_os = "windows")]
                {
                    assets
                        .iter()
                        .find(|a| {
                            a.name
                                .as_ref()
                                .map(|n| {
                                    n.ends_with(".exe")
                                        || n.ends_with(".msi")
                                        || n.ends_with(".zip")
                                })
                                .unwrap_or(false)
                        })
                        .and_then(|a| a.browser_download_url.clone())
                }
                #[cfg(not(any(target_os = "macos", target_os = "windows")))]
                {
                    assets
                        .iter()
                        .find(|a| {
                            a.name
                                .as_ref()
                                .map(|n| {
                                    n.ends_with(".AppImage")
                                        || n.ends_with(".tar.gz")
                                        || n.ends_with(".deb")
                                })
                                .unwrap_or(false)
                        })
                        .and_then(|a| a.browser_download_url.clone())
                }
            })
            .unwrap_or_else(|| html_url.clone());

        Ok(Some(AppRelease {
            version: latest_ver,
            title: release.name.unwrap_or_else(|| format!("v{tag}")),
            body,
            release_notes_zh: zh,
            release_notes_en: en,
            download_url,
            html_url,
            published_at: release.published_at,
            is_prerelease: release.prerelease.unwrap_or(false),
        }))
    } else {
        Ok(None)
    }
}

/// Result of applying a downloaded update package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyUpdateOutcome {
    /// New app was installed and launched; caller should quit.
    Relunched,
    /// Installer / DMG was opened for the user; caller may quit.
    OpenedPackage,
}

fn update_download_dir() -> Result<PathBuf, String> {
    let dir = dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("router-switch")
        .join("updates");
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建更新目录失败: {e}"))?;
    Ok(dir)
}

fn filename_from_url(url: &str) -> String {
    url.split('?')
        .next()
        .and_then(|u| u.rsplit('/').next())
        .filter(|s| !s.is_empty())
        .unwrap_or("router-switch-update.bin")
        .to_string()
}

fn looks_like_installer(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".dmg")
        || lower.ends_with(".pkg")
        || lower.ends_with(".zip")
        || lower.ends_with(".exe")
        || lower.ends_with(".msi")
        || lower.ends_with(".appimage")
        || lower.ends_with(".deb")
        || lower.ends_with(".tar.gz")
}

/// Download a release asset to the local updates cache. `on_progress(downloaded, total)`.
pub fn download_release_asset(
    url: &str,
    mut on_progress: impl FnMut(u64, Option<u64>),
) -> Result<PathBuf, String> {
    if url.trim().is_empty() {
        return Err("下载地址为空".into());
    }
    let name = filename_from_url(url);
    if !looks_like_installer(&name) {
        return Err(format!("下载地址不是安装包: {name}"));
    }
    let dest = update_download_dir()?.join(&name);

    let resp = ureq::get(url)
        .set("User-Agent", "router-switch-app")
        .timeout(Duration::from_secs(120))
        .call()
        .map_err(|e| format!("下载失败: {e}"))?;

    let total = resp
        .header("Content-Length")
        .and_then(|v| v.parse::<u64>().ok());
    let mut reader = resp.into_reader();
    let mut file =
        std::fs::File::create(&dest).map_err(|e| format!("写入更新文件失败: {e}"))?;
    let mut buf = [0u8; 64 * 1024];
    let mut downloaded: u64 = 0;
    loop {
        let n = std::io::Read::read(&mut reader, &mut buf)
            .map_err(|e| format!("读取下载流失败: {e}"))?;
        if n == 0 {
            break;
        }
        std::io::Write::write_all(&mut file, &buf[..n])
            .map_err(|e| format!("写入更新文件失败: {e}"))?;
        downloaded = downloaded.saturating_add(n as u64);
        on_progress(downloaded, total);
    }
    Ok(dest)
}

/// Current `.app` bundle path when running from a packaged macOS app.
#[cfg(target_os = "macos")]
fn current_app_bundle() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    // .../AppName.app/Contents/MacOS/binary
    let macos_dir = exe.parent()?;
    let contents = macos_dir.parent()?;
    let bundle = contents.parent()?;
    if bundle.extension().and_then(|e| e.to_str()) == Some("app") {
        Some(bundle.to_path_buf())
    } else {
        None
    }
}

#[cfg(target_os = "macos")]
fn install_macos_dmg(dmg: &Path) -> Result<ApplyUpdateOutcome, String> {
    let mount_root = update_download_dir()?.join("mnt");
    let _ = std::fs::remove_dir_all(&mount_root);
    std::fs::create_dir_all(&mount_root).map_err(|e| format!("创建挂载目录失败: {e}"))?;

    let attach = std::process::Command::new("hdiutil")
        .args(["attach", "-nobrowse", "-readonly"])
        .arg(dmg)
        .arg("-mountroot")
        .arg(&mount_root)
        .output()
        .map_err(|e| format!("挂载 DMG 失败: {e}"))?;
    if !attach.status.success() {
        return Err(format!(
            "挂载 DMG 失败: {}",
            String::from_utf8_lossy(&attach.stderr)
        ));
    }

    let install_result = (|| -> Result<ApplyUpdateOutcome, String> {
        let mut app_src: Option<PathBuf> = None;
        for entry in walkdir_apps(&mount_root)? {
            app_src = Some(entry);
            break;
        }
        let Some(app_src) = app_src else {
            // Fallback: open the DMG for manual install.
            let _ = std::process::Command::new("open").arg(dmg).spawn();
            return Ok(ApplyUpdateOutcome::OpenedPackage);
        };

        let dest = if let Some(bundle) = current_app_bundle() {
            bundle
        } else {
            PathBuf::from("/Applications").join(
                app_src
                    .file_name()
                    .unwrap_or_else(|| std::ffi::OsStr::new("Router Switch.app")),
            )
        };

        // Replace existing app atomically-ish via ditto into a sibling then rename.
        let dest_parent = dest
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("/Applications"));
        let staging = dest_parent.join(format!(
            ".{}.update",
            dest.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("RouterSwitch.app")
        ));
        let _ = std::fs::remove_dir_all(&staging);
        let ditto = std::process::Command::new("ditto")
            .arg(&app_src)
            .arg(&staging)
            .output()
            .map_err(|e| format!("复制应用失败: {e}"))?;
        if !ditto.status.success() {
            return Err(format!(
                "复制应用失败: {}",
                String::from_utf8_lossy(&ditto.stderr)
            ));
        }
        // Clear quarantine so Gatekeeper doesn't block the new build.
        let _ = std::process::Command::new("xattr")
            .args(["-cr"])
            .arg(&staging)
            .status();

        if dest.exists() {
            let backup = dest_parent.join(format!(
                ".{}.bak",
                dest.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("RouterSwitch.app")
            ));
            let _ = std::fs::remove_dir_all(&backup);
            std::fs::rename(&dest, &backup)
                .map_err(|e| format!("备份旧版本失败: {e}"))?;
            if let Err(err) = std::fs::rename(&staging, &dest) {
                let _ = std::fs::rename(&backup, &dest);
                return Err(format!("替换应用失败: {err}"));
            }
            let _ = std::fs::remove_dir_all(&backup);
        } else {
            std::fs::rename(&staging, &dest)
                .map_err(|e| format!("安装应用失败: {e}"))?;
        }

        let _ = std::process::Command::new("open").arg(&dest).spawn();
        Ok(ApplyUpdateOutcome::Relunched)
    })();

    // Always detach volumes under mount_root.
    if let Ok(entries) = std::fs::read_dir(&mount_root) {
        for entry in entries.flatten() {
            let _ = std::process::Command::new("hdiutil")
                .args(["detach", "-quiet"])
                .arg(entry.path())
                .status();
        }
    }
    let _ = std::fs::remove_dir_all(&mount_root);
    install_result
}

#[cfg(target_os = "macos")]
fn walkdir_apps(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut found = Vec::new();
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
        let entries = std::fs::read_dir(dir).map_err(|e| format!("读取挂载卷失败: {e}"))?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("app") {
                out.push(path);
            } else if path.is_dir() {
                walk(&path, out)?;
            }
        }
        Ok(())
    }
    walk(root, &mut found)?;
    Ok(found)
}

#[cfg(target_os = "macos")]
fn install_macos_zip(zip: &Path) -> Result<ApplyUpdateOutcome, String> {
    let extract_dir = update_download_dir()?.join("extract");
    let _ = std::fs::remove_dir_all(&extract_dir);
    std::fs::create_dir_all(&extract_dir).map_err(|e| format!("创建解压目录失败: {e}"))?;
    let unzip = std::process::Command::new("ditto")
        .args(["-x", "-k"])
        .arg(zip)
        .arg(&extract_dir)
        .output()
        .map_err(|e| format!("解压失败: {e}"))?;
    if !unzip.status.success() {
        return Err(format!(
            "解压失败: {}",
            String::from_utf8_lossy(&unzip.stderr)
        ));
    }
    let apps = walkdir_apps(&extract_dir)?;
    let Some(app_src) = apps.into_iter().next() else {
        let _ = std::process::Command::new("open").arg(zip).spawn();
        return Ok(ApplyUpdateOutcome::OpenedPackage);
    };
    // Reuse ditto path by creating a temp dmg-like install via copy logic:
    // Write a tiny helper by calling ditto to dest directly.
    let dest = if let Some(bundle) = current_app_bundle() {
        bundle
    } else {
        PathBuf::from("/Applications").join(
            app_src
                .file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new("Router Switch.app")),
        )
    };
    let dest_parent = dest
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("/Applications"));
    let staging = dest_parent.join(format!(
        ".{}.update",
        dest.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("RouterSwitch.app")
    ));
    let _ = std::fs::remove_dir_all(&staging);
    let ditto = std::process::Command::new("ditto")
        .arg(&app_src)
        .arg(&staging)
        .output()
        .map_err(|e| format!("复制应用失败: {e}"))?;
    if !ditto.status.success() {
        return Err(format!(
            "复制应用失败: {}",
            String::from_utf8_lossy(&ditto.stderr)
        ));
    }
    let _ = std::process::Command::new("xattr")
        .args(["-cr"])
        .arg(&staging)
        .status();
    if dest.exists() {
        let backup = dest_parent.join(format!(
            ".{}.bak",
            dest.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("RouterSwitch.app")
        ));
        let _ = std::fs::remove_dir_all(&backup);
        std::fs::rename(&dest, &backup).map_err(|e| format!("备份旧版本失败: {e}"))?;
        if let Err(err) = std::fs::rename(&staging, &dest) {
            let _ = std::fs::rename(&backup, &dest);
            return Err(format!("替换应用失败: {err}"));
        }
        let _ = std::fs::remove_dir_all(&backup);
    } else {
        std::fs::rename(&staging, &dest).map_err(|e| format!("安装应用失败: {e}"))?;
    }
    let _ = std::fs::remove_dir_all(&extract_dir);
    let _ = std::process::Command::new("open").arg(&dest).spawn();
    Ok(ApplyUpdateOutcome::Relunched)
}

/// Apply a downloaded update package (DMG / ZIP / EXE / MSI).
pub fn apply_downloaded_update(path: &Path) -> Result<ApplyUpdateOutcome, String> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    #[cfg(target_os = "macos")]
    {
        if name.ends_with(".dmg") {
            return install_macos_dmg(path);
        }
        if name.ends_with(".zip") {
            return install_macos_zip(path);
        }
        if name.ends_with(".pkg") {
            let status = std::process::Command::new("open")
                .arg(path)
                .status()
                .map_err(|e| format!("打开安装包失败: {e}"))?;
            if status.success() {
                return Ok(ApplyUpdateOutcome::OpenedPackage);
            }
            return Err("打开 PKG 安装包失败".into());
        }
    }

    #[cfg(target_os = "windows")]
    {
        if name.ends_with(".exe") || name.ends_with(".msi") {
            std::process::Command::new(path)
                .spawn()
                .map_err(|e| format!("启动安装程序失败: {e}"))?;
            return Ok(ApplyUpdateOutcome::OpenedPackage);
        }
        if name.ends_with(".zip") {
            let _ = std::process::Command::new("explorer").arg(path).spawn();
            return Ok(ApplyUpdateOutcome::OpenedPackage);
        }
    }

    // Generic fallback: open with system handler.
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(path).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", &path.to_string_lossy()])
            .spawn();
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    }
    Ok(ApplyUpdateOutcome::OpenedPackage)
}

/// Returns a realistic sample release for local preview and testing.
pub fn sample_app_release(current_version: &str) -> AppRelease {
    let raw_body = r#"
## 新变化
- 修复：浅色外观下卡片、搜索框和引导页整片发灰发浑。只影响 macOS 26，26 之后的系统上不出现。
- 修复：历史里有图片时横向滚动卡顿。图片卡片现在读捕获时生成的缩略图，滚动时不再解码原图；已有的图片会在启动后后台补齐。
- 激活与许可证的提示文案重写，说清出了什么问题、下一步该做什么。

## What's new
- Fixed: cards, the search field and the onboarding pages looked grey and washed out in Light Appearance. This only affected macOS 26.
- Fixed: horizontal scrolling stuttered when the history contained images. Image cards now read a thumbnail generated at capture time instead of decoding the full image while scrolling; existing images are backfilled in the background at launch.
- Rewrote the licence and activation messages to say what went wrong and what to do next.
"#;

    let (zh, en) = parse_release_notes(raw_body);

    let next_version = if current_version.is_empty() {
        "1.0.1".to_string()
    } else {
        let (major, minor, patch) = match crate::env_checker::parse_semver(current_version) {
            Some((parts, _)) => (parts[0], parts[1], parts[2] + 1),
            None => (1, 0, 1),
        };
        format!("{major}.{minor}.{patch}")
    };

    AppRelease {
        version: next_version,
        title: "Router Switch Update".to_string(),
        body: raw_body.trim().to_string(),
        release_notes_zh: zh,
        release_notes_en: en,
        download_url: "https://github.com/aohun/router-switch/releases".to_string(),
        html_url: "https://github.com/aohun/router-switch/releases".to_string(),
        published_at: Some("2026-08-28T08:00:00Z".to_string()),
        is_prerelease: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_release_notes_structure() {
        let sample = sample_app_release("1.0.0");
        assert_eq!(sample.version, "1.0.1");
        assert_eq!(sample.release_notes_zh.len(), 3);
        assert_eq!(sample.release_notes_en.len(), 3);
        assert!(sample.release_notes_zh[0].contains("浅色外观下卡片"));
        assert!(sample.release_notes_en[0].contains("Fixed: cards"));
    }

    #[test]
    fn test_clean_bullet_line() {
        assert_eq!(clean_bullet_line("- Item 1"), "Item 1");
        assert_eq!(clean_bullet_line("* Item 2"), "Item 2");
        assert_eq!(clean_bullet_line("• Item 3"), "Item 3");
        assert_eq!(clean_bullet_line("1. Item 4"), "Item 4");
    }
}

use serde::{Deserialize, Serialize};
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

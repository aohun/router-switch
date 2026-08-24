# Router Switch

<div align="center">

**Next-Generation AI Gateway Desktop Tool**  
*新一代AI 网关桌面工具*

[English](README.md) · [简体中文](README_ZH.md)

Built with **pure Rust + GPUI + gpui-component**. Ultra-lightweight, native GPU hardware acceleration, zero WebView / Electron / React overhead.

[![GitHub Repo](https://img.shields.io/badge/GitHub-aohun%2Frouter--switch-blue?logo=github)](https://github.com/aohun/router-switch.git)
[![Rust](https://img.shields.io/badge/Rust-1.85%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![GPUI](https://img.shields.io/badge/GUI-GPUI-purple)](https://www.gpui.rs/)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

</div>

---

## 📖 Introduction

**Router Switch** is a next-generation desktop AI gateway and CLI provider switcher built for AI developers. It eliminates the friction of manually modifying configuration files, toggling third-party API providers, and managing credentials across multiple AI CLI tools (such as OpenAI Codex, Anthropic Claude Code, xAI Grok Build, OpenCode, Pi, etc.).

Powered by a native GPU-accelerated rendering engine, Router Switch delivers millisecond-level startup, minimal memory footprint, and a silky smooth 60+ FPS experience without the resource bloat of web-based desktop wrappers.

---

## ✨ Features

### 1. Unified Multi-Tool & Multi-Provider Management
- **Full Suite of AI CLI & Agent Tools**:
  - **OpenAI Codex** (`~/.codex/config.toml`, `~/.codex/auth.json`)
  - **Anthropic Claude Code** (`~/.claude/settings.json`)
  - **xAI Grok Build** (`~/.grok/config.toml`)
  - **OpenCode** (`~/.config/opencode/opencode.json`)
  - **Pi Coding Agent** (`~/.pi/agent/`)
- **One-Click Atomic Switching**: Seamlessly toggle between official accounts and third-party API providers with dual-file atomic writes and automatic rollback protection.

### 2. Smart Clipboard Monitoring & 1-Click Import
- **NewAPI / OneAPI Link Detection**: Automatically parses channel connection JSON copied from NewAPI or OneAPI (`{"_type":"newapi_channel_conn","key":"sk-...","url":"https://..."}`).
- **Automatic Field Extraction**: Populates API Endpoint (Base URL), API Key, provider name, and models into the provider form automatically upon opening.

### 3. Model Discovery & Mapping (Model Mapping)
- **Online Model List Fetching**: Queries available models directly from provider endpoints (`/v1/models`).
- **Flexible Model Aliasing & Configuration**: Custom display names (e.g. mapping `DeepSeek V3` to `deepseek-chat`), context window limits, and reasoning effort levels.

### 4. Local CLI Environment Diagnostics & Conflict Detection
- **CLI Version Detection**: Scans installed AI CLI tools on the system and compares them against the latest remote registry versions.
- **Multi-Installation Conflict Diagnostics**: Detects duplicate binary installations in `PATH` (Homebrew, npm global, Cargo, pip), highlighting the active binary and offering quick upgrade commands.

### 5. Internationalization (i18n) & Theming
- **Bilingual Interface**: Instant switching between Simplified Chinese (`zh-CN`) and English (`en`), with local persistence.
- **Theme Modes**: Supports Light, Dark, and System appearance themes.

### 6. Usage Analytics & Cost Breakdown
- **Granular Insights**: Track request counts, prompt/completion tokens, and estimated cost across Daily, Monthly, and Workspace Project views.

### 7. Workspace Customization & Desktop Integration
- **Navigation Customization**: Drag-and-drop reordering and visibility toggles for AI tools in the sidebar.
- **Desktop Preferences**: Launch on startup and minimize-to-tray on window close.

---

## 🛠️ Tech Stack & Dependencies

### Core Language & Runtime
- **[Rust](https://www.rust-lang.org/)** (1.85+ / Edition 2021 & 2024): Memory safety, high concurrency, and unmatched native performance.

### GUI Rendering & Component Library
- **[GPUI](https://www.gpui.rs/)** (v0.2.2): High-performance GPU-accelerated UI framework developed by the Zed team (rendering directly via Metal / Vulkan / Direct3D).
- **[gpui-component](https://github.com/longbridge/gpui-component)** (v0.5.1): Production-grade native component library by Longbridge providing Button, Input, Select, Dialog, Notification, Tile, Tag, and more.
- **gpui-component-assets**: Embedded Lucide icon suite and custom SVG assets.

### Persistence & Data Layer
- **[rusqlite](https://github.com/rusqlite/rusqlite)** (v0.32 bundled): Embedded SQLite database as the single source of truth (`~/.router-switch/app.db`).
- **[serde](https://serde.rs/) / serde_json / toml**: Fast, robust serialization and deserialization.

### System & Networking
- **[rust-i18n](https://github.com/longbridge/rust-i18n)** (v3.1.5): Compile-time internationalization.
- **[ureq](https://github.com/algesten/ureq)** (v2.10): Lightweight synchronous HTTP client for model list retrieval and registry queries.
- **[dirs](https://github.com/dirs-dev/dirs-rs)** (v6): Cross-platform standard directory resolution.

---

## 🏛️ Architecture

```
router-switch/
├── Cargo.toml               # Workspace root configuration
├── crates/
│   ├── domain/              # Pure domain logic: form validation, config templates, clipboard parser, env inspection
│   ├── adapters-codex/      # Codex live config file read/write adapter (~/.codex)
│   ├── adapters-claude/     # Claude Code live config adapter (~/.claude)
│   ├── adapters-grok/       # Grok Build live config adapter (~/.grok)
│   ├── adapters-opencode/   # OpenCode live config adapter (~/.config/opencode)
│   ├── adapters-pi/         # Pi live config adapter (~/.pi/agent)
│   ├── store/               # SQLite SSOT database storage (~/.router-switch/app.db)
│   ├── session/             # Workspace orchestration: provider lifecycle & live atomic updates
│   ├── ui/                  # GPUI views, themes, i18n, and interactive UI components
│   └── app/                 # Main desktop application binary entry point
```

---

## 📂 Configuration & Data Directories

| Path | Description |
|---|---|
| `~/.router-switch/app.db` | Application SQLite database (providers, active states, settings) |
| `~/.codex/config.toml` | Live Codex endpoints & model configuration |
| `~/.codex/auth.json` | Live Codex authentication tokens and API keys |
| `~/.claude/settings.json` | Live Claude Code environment variables and settings |
| `~/.grok/config.toml` | Live Grok Build endpoint and API key |
| `~/.config/opencode/opencode.json` | Live OpenCode configuration |
| `~/.pi/agent/` | Live Pi configuration and model mappings |

*Note: You can override storage paths using environment variables such as `ROUTER_SWITCH_HOME` and `CODEX_HOME`.*

---

## 🚀 Quick Start

### Prerequisites
- **Rust Toolchain**: `rustc 1.85.0` or higher (recommended: `rustc 1.93+`)
- **Operating System**: macOS (Apple Silicon / Intel) or Windows 10/11

### Build & Run

```bash
# Clone the repository
git clone https://github.com/aohun/router-switch.git
cd router-switch

# Run the desktop application
cargo run -p router-switch

# Run the test suite
cargo test --all
```

---

## 📄 License

This project is licensed under the [MIT License](LICENSE). Contributions, issues, and feature requests are welcome!

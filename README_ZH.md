# Router Switch

<div align="center">

**新一代AI 网关桌面工具**  
*Next-Generation AI Gateway Desktop Tool*

[English](README.md) · [简体中文](README_ZH.md)

基于 **纯 Rust + GPUI + gpui-component** 构建，极致轻量，原生 GPU 硬件加速，零 WebView / Electron / React 依赖。

[![GitHub Repo](https://img.shields.io/badge/GitHub-aohun%2Frouter--switch-blue?logo=github)](https://github.com/aohun/router-switch.git)
[![Rust](https://img.shields.io/badge/Rust-1.85%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![GPUI](https://img.shields.io/badge/GUI-GPUI-purple)](https://www.gpui.rs/)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

</div>

---

## 📖 项目简介

**Router Switch** 是专为 AI 开发者打造的新一代 AI 网关桌面工具与配置中枢。它解决了在各大 AI CLI 工具（如 OpenAI Codex、Anthropic Claude Code、xAI Grok Build、OpenCode、Pi 等）中频繁手动修改配置文件、切换第三方 API 供应商以及管理多端点凭据的繁琐痛点。

通过原生 GPU 渲染引擎，Router Switch 提供了毫秒级冷启动、极低内存占用与 60+ FPS 流畅交互体验，彻底告别传统前端桌面框架的资源开销。

---

## ✨ 已实现功能

### 1. 多工具多供应商统一管理
- **支持全系列主流 AI 工具**：
  - **OpenAI Codex** (`~/.codex/config.toml`, `~/.codex/auth.json`)
  - **Anthropic Claude Code** (`~/.claude/settings.json`)
  - **xAI Grok Build** (`~/.grok/config.toml`)
  - **OpenCode** (`~/.config/opencode/opencode.json`)
  - **Pi Coding Agent** (`~/.pi/agent/`)
- **一键原子切换**：支持官方登录认证与第三方 API 供应商之间的一键平滑切换，提供双文件写入原子保护与错误自动回滚机制。

### 2. 剪贴板智能识别与一键导入
- **NewAPI / OneAPI 链接识别**：自动监控并解析剪贴板中导出的渠道连接 JSON（如 `{"_type":"newapi_channel_conn","key":"sk-...","url":"https://..."}`）。
- **自动填充与提取**：在新建供应商页面自动提取 API 端点 (Base URL)、API Key、服务商名称与模型列表，无需手动复制粘贴。

### 3. 模型探测与模型映射 (Model Mapping)
- **在线模型获取**：支持从第三方端点一键拉取可用的模型列表 (`/v1/models`)。
- **灵活模型映射**：支持自定义模型别名（如将 `DeepSeek V3` 映射至底层 `deepseek-chat`）、上下文窗口大小 (Context Window) 与思考等级 (Reasoning Effort)。

### 4. 本地 CLI 环境检测与多安装冲突诊断
- **CLI 版本诊断**：自动扫描系统已安装的 AI CLI 工具及当前运行版本。
- **多重安装冲突检测**：检测 `PATH` 中多重安装路径（如 Homebrew、npm 全局路径、Cargo、pip 冲突），明确标出命令行默认调用的二进制，并支持一键升级指令。

### 5. 国际化 (i18n) 与多主题 (Theming)
- **多语言即时切换**：内置简体中文 (`zh-CN`) 与英文 (`en`) 支持，切换立即生效并本地持久化。
- **丰富主题支持**：提供浅色 (Light)、深色 (Dark) 及跟随系统 (System) 主题模式。

### 6. 用量统计与可视化 (Usage Analytics)
- **多维度统计**：支持按日 (Daily)、按月 (Monthly)、按项目 (Projects) 查看请求数、输入/输出 Token 及金额花销明细。

### 7. 自定义工作区与桌面集成
- **左侧导航栏排序与管理**：支持自定义主页面 AI 工具入口的显隐与拖拽排序。
- **桌面行为**：支持开机自启、关闭时最小化到系统托盘等功能。

---

## 🛠️ 技术栈与依赖组件库

### 核心语言与运行时
- **[Rust](https://www.rust-lang.org/)** (1.85+ / Edition 2021 & 2024)：提供内存安全、高并发与极致性能。

### GUI 渲染与组件库
- **[GPUI](https://www.gpui.rs/)** (v0.2.2)：由 Zed 团队开发的 GPU 加速原生 UI 框架，采用与 Web 相似的 Flexbox 布局但直接基于 Metal/Vulkan 硬件加速渲染。
- **[gpui-component](https://github.com/longbridge/gpui-component)** (v0.5.1)：Longbridge 开源的高品质 GPUI 原生组件库，提供 Button、Input、Select、Dialog、Notification、Tile、Tag 等基础与高级 UI 控件。
- **gpui-component-assets**：内置图标集与 Lucide 矢量图标。

### 本地持久化与数据模型
- **[rusqlite](https://github.com/rusqlite/rusqlite)** (v0.32 bundled)：内置 SQLite 数据库作为单一数据源 (SSOT)。
- **[serde](https://serde.rs/) / serde_json / toml**：全格式序列化与反序列化支持。

### 系统集成与网络
- **[rust-i18n](https://github.com/longbridge/rust-i18n)** (v3.1.5)：编译期多语言本地化方案。
- **[ureq](https://github.com/algesten/ureq)** (v2.10)：轻量级同步 HTTP 请求库，用于模型列表拉取与版本检测。
- **[dirs](https://github.com/dirs-dev/dirs-rs)** (v6)：跨平台标准目录路径解析。

---

## 🏛️ 工程分层架构

```
router-switch/
├── Cargo.toml               # Workspace 根配置
├── crates/
│   ├── domain/              # 纯领域模型：表单校验、配置模板生成、剪贴板解析、CLI 环境探测
│   ├── adapters-codex/      # Codex live 配置文件读写适配器 (~/.codex)
│   ├── adapters-claude/     # Claude Code live 配置适配器 (~/.claude)
│   ├── adapters-grok/       # Grok Build live 配置适配器 (~/.grok)
│   ├── adapters-opencode/   # OpenCode live 配置适配器 (~/.config/opencode)
│   ├── adapters-pi/         # Pi live 配置适配器 (~/.pi/agent)
│   ├── store/               # SQLite SSOT 数据存储 (~/.router-switch/app.db)
│   ├── session/             # 业务编排层：供应商生命周期、切换与 Live 原子写入
│   ├── ui/                  # GPUI 视图、主题渲染、国际化与用户交互逻辑
│   └── app/                 # 桌面应用程序入口 main.rs
```

---

## 📂 本地数据与配置文件目录

| 路径 | 说明 |
|---|---|
| `~/.router-switch/app.db` | 应用持久化数据库（供应商列表、当前激活项、偏好设置） |
| `~/.codex/config.toml` | Codex 激活端点与模型配置 |
| `~/.codex/auth.json` | Codex 登录凭据与 API Key |
| `~/.claude/settings.json` | Claude Code 激活环境变量与配置 |
| `~/.grok/config.toml` | Grok Build 激活端点与 Key |
| `~/.config/opencode/opencode.json` | OpenCode 激活配置 |
| `~/.pi/agent/` | Pi 激活配置与模型映射 |

*提示：可通过设置 `ROUTER_SWITCH_HOME` 与 `CODEX_HOME` 等环境变量自定义数据与配置文件存储路径。*

---

## 🚀 快速开始

### 环境要求
- **Rust 工具链**：`rustc 1.85.0` 或更高版本（推荐 `rustc 1.93+`）
- **操作系统**：macOS (Apple Silicon / Intel)、Windows 10/11

### 编译与运行

```bash
# 克隆仓库
git clone https://github.com/aohun/router-switch.git
cd router-switch

# 运行桌面客户端
cargo run -p router-switch

# 运行全套单元测试与集成测试
cargo test --all
```

---

## 📄 开源许可

本项目基于 [MIT License](LICENSE) 开源。欢迎提交 Issue 与 Pull Request 共同完善！

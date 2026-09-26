# Router Switch Agent 指南 & 开发规范

本项目是基于纯 Rust + GPUI 构建的高性能 AI 网关桌面工具与服务商配置中枢。所有参与此项目的 AI Agent 及开发者必须严格遵守以下规范与安全准则。

---

## 📚 0. 参考项目（Wake）

会话管理与 Insights 仪表盘的 **UX / 交互 / 统计口径** 以 [Wake](https://github.com/) 为对照实现：

| 项 | 路径 |
|----|------|
| 本地仓库 | `/Users/wayne/Desktop/git/Wake` |
| 源码快照（可读对照） | `/tmp/Wake-src`（`crates/wake`、`crates/wake-core`） |
| 三栏会话工作台 | `wake/src/workbench.rs`（侧栏范围筛选 + 中栏列表 + 右栏 transcript） |
| 会话索引 / 适配器 | `wake-core/src/db.rs`、`wake-core/src/adapters/*`、`scanner.rs` |
| Insights 配色 / Agent Tokens | `wake/src/theme.rs`（系统蓝 `#0A84FF` / `#4C8DFF`）、`workbench.rs` usage board |

**对齐约定：**

- 会话管理目标形态：Wake 三栏（全部 / 已收藏 / Agent / 项目 → 会话列表 → transcript + 收藏/置顶/复制路径/恢复）。
- 仪表盘（原「统计」菜单名保持 **仪表盘**）图表色与 Tokens 榜口径对齐 Wake；扫描数据仍走本仓 `tokens-core`。
- 实现时可参考 Wake，但 **不得** 把 Wake 用户库、会话文件或密钥拷进本仓库；本机索引落在 `~/.router-switch/`。

---

## 🔒 1. 安全与凭据隔离规范（最高优先级）

在提交代码、执行测试或触发 GitHub Actions 工作流时，**严禁将本机的真实服务商配置、API Key、Token 或数据库文件提交至代码仓库**。

### 1.1 数据存储与配置隔离原则
- **真实配置绝不在仓库内**：
  - 应用的运行时数据库必须持久化在系统用户家目录（`~/.router-switch/app.db`）。
  - 各 AI 工具的 Live 配置文件均位于各自的家目录（`~/.codex/`、`~/.claude/`、`~/.grok/`、`~/.config/opencode/`、`~/.pi/agent/`、`~/.zcode/v2/` 等），绝不可在项目仓库根目录下生成或提交。
- **单元测试与 Mock 数据安全**：
  - 所有单元测试、示例数据与文档中的 API Key 必须使用通用 Mock 占位符（例如 `sk-mock-key-12345`、`sk-ant-test`、`sk-ds-123`），**绝对禁止拷贝或残留开发机上的真实密钥**（如 `sk-cchost-...` 等私有渠道 Key）。
  - 域名必须使用示例域名（如 `https://api.example.com/v1`）或官方通用端点。

### 1.2 提交前审查清单 (Pre-commit Audit)
每次执行 `git commit` 或打 Tag 前，必须确认：
1. 运行 `git status` 检查是否有意外生成的本地临时文件、证书、密钥或本地数据库。
2. 运行 `git diff --cached` 检查暂存区改动，确保没有硬编码任何私有服务商配置或密钥。
3. 检查 `.gitignore` 确保已覆盖以下类型：
   - 本地数据库：`*.db`, `*.db-shm`, `*.db-wal`, `*.sqlite`, `*.sqlite3`
   - 环境变量与本地配置：`.env`, `.env.*`, `*.local`, `config.json`, `auth.json`, `settings.json`
   - 证书与私钥：`*.pem`, `*.key`, `*.crt`, `*.pfx`, `*.p12`
   - 客户端配置目录镜像：`.router-switch/`, `.zcode/`, `.codex/`, `.claude/`, `.grok/`, `.opencode/`, `.pi/`

### 1.3 CI / GitHub Actions 隔离
- GitHub Actions Workflow 在干净的隔离容器/虚拟机中运行，只编译代码仓库内的源文件。
- 打包脚本（如 `scripts/bundle-dmg.sh`、Windows Inno Setup 等）仅打包编译输出的二进制文件及 `assets/` 中的公共静态资源，绝不打包任何用户本地数据或私有路径。

---

## 🏛️ 2. 项目架构与分层职责

```
router-switch/
├── Cargo.toml               # Workspace 根配置与依赖版本
├── crates/
│   ├── domain/              # 纯领域模型：表单、模型映射、剪贴板解析、深度链接协议（无外部 I/O 依赖）
│   ├── adapters-codex/      # Codex Live 配置文件读写适配器 (~/.codex)
│   ├── adapters-claude/     # Claude Code Live 配置适配器 (~/.claude)
│   ├── adapters-grok/       # Grok Build Live 配置适配器 (~/.grok)
│   ├── adapters-opencode/   # OpenCode Live 配置适配器 (~/.config/opencode)
│   ├── adapters-pi/         # Pi Live 配置适配器 (~/.pi/agent)
│   ├── adapters-zcode/      # ZCode Live 配置适配器 (~/.zcode/v2/config.json)
│   ├── cursor-gateway/      # Cursor 本地代理与中间人网关服务
│   ├── tokens-core/         # 多 Agent 会话扫描与 Insights 聚合（对齐 Wake 统计）
│   ├── store/               # SQLite SSOT（~/.router-switch/app.db；含会话索引 / 收藏置顶）
│   ├── session/             # 业务编排 + Wake 风格会话索引 / transcript 加载
│   ├── ui/                  # GPUI 视图（含 Route::Sessions 三栏会话管理）
│   └── app/                 # 桌面主程序入口 main.rs
├── assets/                  # 应用元数据（Info.plist、图标等）
└── scripts/                 # 跨平台打包脚本（bundle-dmg.sh 等）
```

---

## 🛠️ 3. 开发与测试准则

- **测试先行**：每次增加或修改适配器、表单逻辑或网关逻辑后，必须运行全量测试：
  - `cargo test --all`
  - `cargo fmt --check`
- **UI 开发规范**：实现 UI 界面时，优先使用 [GPUI Component](https://longbridge.github.io/gpui-component)。
- **国际化维护**：若在 UI 中增加新文本或修改文案，必须同步更新 `crates/ui/locales/zh-CN.yml` 与 `crates/ui/locales/en.yml`。
- **术语规范**：统一使用“**服务商**”（Provider），不再使用“供应商”。

---

## 🚀 4. CI/CD 与发布规范

- **版本号同步**：发布新版本时，统一更新根目录 `Cargo.toml` 下的 `workspace.package.version`，各子 Crate 继承 `version.workspace = true`。
- **macOS Runner 配置**：
  - macOS arm64 和 x86_64 均采用 `runs-on: macos-14`（Apple Silicon Runner）。
  - x86_64 通过 Rust 交叉编译工具链 `--target x86_64-apple-darwin` 构建，严禁使用已弃用的 `macos-13`。

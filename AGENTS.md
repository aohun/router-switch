# Router Switch Agent 指南 & 开发规范

本项目是基于纯 Rust + GPUI 构建的高性能 AI 网关桌面工具与服务商配置中枢。所有参与此项目的 AI Agent 及开发者必须严格遵守以下规范与安全准则。

---

## 📚 0. 参考项目

### 0.1 Wake（会话管理 / Insights）

会话管理与 Insights 仪表盘的 **UX / 交互 / 统计口径** 以 Wake 为对照实现：

| 项 | 路径 |
|----|------|
| 本地仓库 | `/Users/wayne/Desktop/git/Wake` |
| 源码快照（可读对照） | `/tmp/Wake-src`（`crates/wake`、`crates/wake-core`） |
| 三栏会话工作台 | `wake/src/workbench.rs`（侧栏范围筛选 + 中栏列表 + 右栏 transcript） |
| 会话索引 / 适配器 | `wake-core/src/db.rs`、`wake-core/src/adapters/*`、`scanner.rs` |
| Insights 配色 / Agent Tokens | `wake/src/theme.rs`（系统蓝 `#0A84FF` / `#4C8DFF`）、`workbench.rs` usage board |

**对齐约定：**

- 会话管理目标形态：Wake 三栏（全部 / 已收藏 / Agent / 项目 → 会话列表 → transcript + 收藏/置顶/复制路径/恢复）。
- **用量**（`Route::Dashboard`，侧栏文案；原「仪表盘 / 统计」）图表色与 Tokens 榜口径对齐 Wake；扫描数据仍走本仓 `tokens-core`。侧栏顺序：供应商 → 网关 → 路由 → 用量。
- 实现时可参考 Wake，但 **不得** 把 Wake 用户库、会话文件或密钥拷进本仓库；本机索引落在 `~/.router-switch/`。

### 0.2 AstrLink（网关中枢 / 供应商）

侧栏 **供应商**（网关上游注册表，`Route::ApiProviders`）以 [AstrLink](https://github.com/) 为对照实现，**区别于**各 Agent 侧栏里的「按应用同步 Live 配置」服务商页：

| 项 | 路径 |
|----|------|
| 本地仓库 | `/Users/wayne/Desktop/git/AstrLink` |
| 列表 / 编辑主界面 | `apps/desktop/src/ServiceManager.tsx` |
| 数据模型 / 预置类型 | `service-model.ts`、`service-presets.ts` |
| 行组件 / 模型编辑 / 连通性测试 | `components/ServiceListRow.tsx`、`ServiceModelsEditor.tsx`、`ServiceTestDialog.tsx` |
| 文档 | `docs/guides/provider-testing.md`、`pay-as-you-go-providers.md` |

**对齐约定：**

- 导航位置：侧栏独立菜单 **供应商**（`Route::ApiProviders`），列表交互对齐 AstrLink：状态分段（全部 / 已启用 / 已停用）、搜索、按模型筛选、拖拽排序、启用开关、测试 / 编辑 / 更多菜单。
- 能力面：创建 / 编辑（连接 · 模型 · 协议 · 失败处理）、订阅类 OAuth / Device Code、HTTP 密钥、模型探测、用量 / 费用展示——逐步对齐 AstrLink，数据落在 `~/.router-switch/app.db` 的 `api_providers` 表，**不得**把 AstrLink 本机密钥或数据库拷进本仓库。
- 术语：网关上游注册表对外文案用「**供应商**」（`api_providers.*`）；各 Agent Live 配置页仍用「**服务商**」（`provider.*`），两套命名空间分开。
- **列表顺序 = 上游优先级**（对齐 AstrLink `service-order` / `StoreResolver`）：
  - UI：行左侧 Grip + 全局序号；拖拽 / ↑↓ 调整顺序；筛选视图重排时只改可见项在全局序列中的槽位（停用项仍占位）。
  - 持久化：`api_providers.sort_index`（及 `data_json` 内字段）由 `Store::replace_api_provider_order` 原子重写；全量 ID 必须一一对应。
  - 路由真相源：`domain::resolve_upstream_candidates`（按 `sort_index`、跳过停用、按模型/协议过滤）；`Workspace::resolve_api_upstream_candidates` 供后续网关故障转移接入。对照：`AstrLink` 的 `OrderedList` / `use-service-order` / `store_resolver.go`。
- **网关**（`Route::Gateway`，侧栏文案；位于供应商正下方、路由上方）对齐 AstrLink `AccessTokenManager` / `accesstoken`：
  - UI：创建 / 刷新 / API 地址复制 / 搜索 / 令牌行（hint · 创建时间 · 今日/累计 Token · CC Switch 导入 · 删除 · 复制密钥）+ 删除确认（末令牌警告）。
  - 数据：`local_access_tokens` + `local_access_token_secrets` + bootstrap 表；前缀 `rsw_`；限 100；首次启动生成「默认令牌」。
  - Session：`list/create/reveal/delete/authenticate`；`gateway_api_url` / `is_gateway_ready`。
  - 对照：`AstrLink` 的 `AccessTokenManager.tsx`、`CCSwitchImportDialog.tsx`、`core/internal/accesstoken`。用量聚合与网关 ingress 鉴权消费另开任务（本阶段 UI + 持久化 + hash 鉴权 API）。
- **路由**（`Route::SmartRouting`，侧栏文案；位于网关正下方、用量上方）对齐 AstrLink `RouteManager` / `RoutingSettingsPanel`：五页签（模型重定向 / 恢复与重试 / 错误规则 / 会话粘性 / 转发身份）+ 500ms 自动保存；配置落在 `kv.routing_settings`（`domain::RoutingSettings`）。网关运行时消费（重定向 / 故障转移 / 粘性 / 身份头）另开任务，本阶段仅 UI + 持久化。
  - **恢复与重试页（强制 1:1，对照 AstrLink `RoutingSettingsPanel` recovery + `FailoverEditor` / `FailurePolicyEditor`）**：三个 Panel 分组，**不得**改成扁平 Checkbox + 三段策略按钮。
    1. **失败恢复与切换**（`orderTitle` / `orderHint`）：Switch「失败后允许自动换 API 提供商」+ `globalOffHint`；两列 Field——下拉「失败后怎么做」（`AABBCC：先重试，再切换` / `ABC：失败后换下一家`）+「一次请求最多尝试几次」及 `maxAttemptsHint`。
    2. **重试次数与等待时间**：hint 随策略切换（`onceHint` / `allServicesHint`）；最多重试几次 / 第一次重试等待 / 最长重试等待（含 hint）。
    3. **推理内容修复**：三个 Switch（思考签名 / OpenAI 推理 / 函数输出密文）+ 各自 hint；默认 thinking/reasoning 开、function 关（对齐 AstrLink `!== false` / `=== true`）。
    数值与开关改动走 500ms 自动保存，**不得**再放「应用数值修改」按钮。
  - **错误规则页（强制 1:1，对照 AstrLink `FailureRulesEditor`）**：页内标题「重试规则」+ HelpCircle 说明 +「重置此组设置」；表格三列（错误类型 / 允许重试 Switch / HTTP 行删除 X）；网络错误与超时固定前两行，HTTP 状态按码排序、可删；底栏输入 `400-599` +「添加规则」。Switch 语义：`action !== stop` 为开，开→`retry_and_failover`、关→`stop`。**不得**用四段动作按钮代替 Switch 表。

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
│   ├── domain/              # 纯领域模型：表单、模型映射、剪贴板解析、深度链接、供应商 / 路由配置（无外部 I/O 依赖）
│   ├── adapters-codex/      # Codex Live 配置文件读写适配器 (~/.codex)
│   ├── adapters-claude/     # Claude Code Live 配置适配器 (~/.claude)
│   ├── adapters-grok/       # Grok Build Live 配置适配器 (~/.grok)
│   ├── adapters-opencode/   # OpenCode Live 配置适配器 (~/.config/opencode)
│   ├── adapters-pi/         # Pi Live 配置适配器 (~/.pi/agent)
│   ├── adapters-zcode/      # ZCode Live 配置适配器 (~/.zcode/v2/config.json)
│   ├── cursor-gateway/      # Cursor 本地代理与中间人网关服务
│   ├── tokens-core/         # 多 Agent 会话扫描与 Insights 聚合（对齐 Wake 统计）
│   ├── store/               # SQLite SSOT（~/.router-switch/app.db；含会话索引 / 供应商 / 路由设置）
│   ├── session/             # 业务编排 + Wake 风格会话索引 / transcript 加载
│   ├── ui/                  # GPUI 视图（用量 / 供应商 / 路由 / Route::Sessions）
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
- **对照产品 1:1 复刻（强制）**：凡 AGENTS.md §0 标明「对齐 AstrLink / Wake」的页面与组件，交付时必须做到 **样式 + 交互 + 文案层级 1:1**（布局结构、控件类型、选中/悬停态、尺寸、图标、间距），不得自行换成「近似」方案（例如用 Checkbox 代替 AstrLink Switch、用分段按钮代替 Select、用大号默认按钮代替 `sm`、缺少 clearable 输入等）。以对照仓库源码与本仓 AGENTS 对该页的专项约定为准；专项约定写明「对齐 AstrLink 某组件」时，以该对照实现为准，不得用本仓历史近似稿覆盖。
- **按钮与表单控件尺寸（强制，对齐 AstrLink `size="sm"`）**：
  - 工具栏主操作、行内操作、弹窗 / Dialog 底部「取消 / 确认」等按钮默认使用 `.small()`（约 28–32px 高），**禁止**使用默认大号按钮。
  - 弹窗与工具栏内的单行 `Input` 同步使用 `.small()`，与按钮高度对齐；需要清除的数字/搜索框加 `.cleanable(true)`。
  - 仅图标的辅助按钮可用 `.xsmall()`（如复制 API 地址、关闭弹窗）。
  - 对照：AstrLink `Button size="sm"` / `Input h-8`；本仓网关页与路由「恢复与重试」为范例。
- **国际化维护**：若在 UI 中增加新文本或修改文案，必须同步更新 `crates/ui/locales/zh-CN.yml` 与 `crates/ui/locales/en.yml`。
- **术语规范**：网关上游注册表（`api_providers.*` / `Route::ApiProviders`）对外文案用「**供应商**」；各 Agent Live 配置（`provider.*`）仍用「**服务商**」。
- **完成后必须重启应用（强制）**：凡改动可影响桌面端行为的任务（UI / session / store / domain / gateway / 文案资源等），在一轮实现或修复结束后，Agent **必须**结束旧进程并用最新代码重新启动应用，便于用户立刻验证，不得只编译不启动：
  1. `pkill -f 'target/debug/router-switch' 2>/dev/null || true`（或等价方式结束本仓 debug 实例）
  2. 在仓库根目录执行：`cargo run -p router-switch`
  3. 确认日志出现 `Running \`target/debug/router-switch\`` 后再向用户汇报完成

---

## 🚀 4. CI/CD 与发布规范

- **版本号同步**：发布新版本时，统一更新根目录 `Cargo.toml` 下的 `workspace.package.version`，各子 Crate 继承 `version.workspace = true`。
- **macOS Runner 配置**：
  - macOS arm64 和 x86_64 均采用 `runs-on: macos-14`（Apple Silicon Runner）。
  - x86_64 通过 Rust 交叉编译工具链 `--target x86_64-apple-darwin` 构建，严禁使用已弃用的 `macos-13`。

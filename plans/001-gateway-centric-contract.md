# Plan 001: 网关中心化合同（身份、存储、写回语义）

> This plan is an outcome contract, not a step-by-step script. Understand the
> requirement and the recorded decisions, then design the implementation
> yourself against the live code. Run milestone validations as you go only if
> you are also the verifier — a delegated executor implements only, and
> verification happens outside its session. Stop on any STOP condition. When
> complete, update this plan in `plans/README.md`.
>
> Drift check: `git diff --stat 7e840ea..HEAD -- crates/domain crates/store crates/session crates/cursor-gateway/src/compat Cargo.toml plans/`

## Status

- Priority: P1
- Effort: M
- Risk: HIGH
- Depends on: none
- Category: migration
- Planned at: `7e840ea`, 2026-09-26

## Requirement

Router Switch 今天的主轴是「按 App 切换服务商并写 live 配置」；CompatGateway 只在协议不匹配时偶发启动，且用**上游 API Key**做 Bearer 匹配选路（`CompatRegistry::resolve`）。产品要改为：**本机网关是唯一推理入口**；用户配置一次提供商与令牌后，adapters 只把各 CLI 指到网关。

本 plan **不实现运行时行为**，只冻结后续并行实现必须遵守的合同：身份模型、SQLite 表形状、适配器写回语义、MVP 功能边界。完成后，`002`/`003` 可并行开工且不互相猜接口。

## Decisions & tradeoffs

- **产品方向 = 方案 A（网关中心 + adapters 写回网关）**: 选定重写 RS 产品中心，不采用双 App 协作，不把 AstrLink Go Core 嵌为 sidecar。Rejected: 双客户端协作 — 违背「填写一次配置」；Rejected: 嵌入 AstrLink Core — 双运行时 + RelayKit AGPL 运维成本过高。
  Based on: 产品决策（用户确认方案 A）。

- **不整仓合并 AstrLink**: 只参考其产品行为与协议习惯；实现留在 Rust/RS。Rejected: 移植 `core/` Go 模块进 RS。
  Based on: AstrLink 栈为 Go+Tauri+React；RS 为纯 Rust+GPUI。

- **MVP 不做网关内订阅 OAuth（Codex/Claude/Grok）** `(decided while planning)`: 第一期提供商仅 **API Key / NewAPI / OpenAI 兼容** 类。订阅可保留「官方直连/现有登录」为非网关路径，或暂不宣传。Rejected: MVP 就做网关内 Device Code/OAuth — XL 且阻塞写回闭环。
  Based on: AstrLink 订阅在 `core/internal/subscription` / `accountauth` 为 XL；方案 A 原文将订阅级能力后置。

- **鉴权模型：本地访问令牌 ≠ 上游 Key**: Agent/CLI 只持有网关签发的 token；上游凭据仅存在 store/网关出站。Rejected: 继续用上游 Key 当 Bearer 选 target（现状 `resolve` 精确匹配 `CompatTarget.api_key`）。
  Based on: `crates/cursor-gateway/src/compat/mod.rs:103-113`（按上游 key 匹配，失败则 first target）。

- **网关 always-on，固定环回端口**: 应用启动（或用户开启网关）即监听；默认端口产品化（可沿用 `8787` 或新约定，但须稳定可复制）。Rejected: 仅在 `routing_plan` 判定协议不匹配时才 `register_compat_target` 启动。
  Based on: `crates/session/src/lib.rs:2017`（`routing_plan`）与 `2088-2105`（按需 start）。

- **适配器写回语义（合同）**: 对已启用「经网关」的 App，live 配置写入 `base_url|baseURL|ANTHROPIC_BASE_URL… = http://127.0.0.1:<gateway_port>/…`，`api_key|Authorization = <本地访问令牌>`；**DB 中 providers 仍保存真实上游**。Rejected: live 文件继续写入上游 URL+上游 Key（现状主路径）。
  Based on: 当前 live 改写仅在 compat 路径替换 host（`session` `gateway_base_url` / `register_compat_target` 调用点约 `1735+`）。

- **共享 schema 只在本 plan 定稿**: `access_tokens`、最小 `request_ledger`（或等价名）、网关设置（port/enabled）的表字段与 Rust 类型由本 plan 里程碑产出；`002`/`003` 不得私自改列。Rejected: 各平行 plan 各自加迁移。
  Based on: 并行安全准则（共享面属合同 plan）。

- **Cursor MITM / 隐私 / auto 分类 / Gemini+RelayKit：本 roadmap 合同外**: 不阻塞 MVP。Cursor MITM 保持可选旁路，不得当作通用入口。
  Based on: 方案 A 原文；MITM 限 `*.cursor.sh`（`harness/proxy.rs`）。

- **请求账本最小集**: MVP 只要求网关侧能写入「令牌 id、模型、上游尝试结果、用量字段（若可得）」；完整 UI/轨迹浏览器留给 `005`。Rejected: MVP 就做齐 AstrLink 式 trajectory 全页。

## Direction

架构边界（合同级）：

```text
CLI/Agent --(local access token)--> Gateway (always-on)
                                      |-- authz by access_tokens
                                      |-- pick upstream from providers (+ later routes)
                                      |-- forward (Chat / Messages / Responses)
                                      '-- append request_ledger
adapters/*  -- write live -->  base_url=gateway, api_key=local token
store       -- SSOT --> providers, tokens, gateway settings, ledger
```

### Milestone 1: 书面合同落地为可编译的类型与迁移草稿

仓库内存在**单一事实来源**描述：访问令牌字段、网关设置、最小 ledger 列、以及「PointToGateway」写回所需的结构化参数（port、token secret 一次展示策略、path 前缀规则）。以可编译的 Rust 类型 + SQLite migration（即使尚无完整业务逻辑）或等价 schema 模块为准。  
Validation: `cargo check -p store`（或承载 schema 的 crate）-> exit 0；`cargo check -p domain` -> exit 0。

### Milestone 2: 适配器写回合同测试夹具

存在**不依赖真实网关进程**的契约测试：给定「gateway base + local token」，对至少 2 个 App 适配器（建议 Codex + Claude）断言将写入的 live 字段是网关指针而非上游 URL/Key。  
Validation: `cargo test -p domain -- <契约测试过滤器>` 或 `cargo test -p adapters-codex -- <…>` 与 `cargo test -p adapters-claude -- <…>` -> exit 0（以实际落地的测试名为准，但必须失败于「仍写上游」的实现）。

### Milestone 3: README/计划索引与并行边界声明

`plans/README.md` 中 001 可标为可执行完成；`002`/`003` 的 in-scope 文件集合在合同中无交集争议。  
Validation: 人工确认 Scope 节与 README 并行组描述一致（本 milestone 无强制命令；由 verifier 勾选 Done）。

## Landmines

- **`CompatRegistry::resolve` 的 first-target 回退**会在多 App 空 key 时串台（`compat/mod.rs:103-113`）。新鉴权必须**拒绝未知令牌**，禁止静默 first-wins。
- **`routing_plan` 按「协议是否与 App 原生方言一致」决定是否走 compat**（`session/src/lib.rs:2017`）。网关中心化后，该分支条件应变为「App 是否启用经网关」，否则大量「方言匹配」流量仍直连上游，产品叙事破裂。
- **ZCode 等不走 compat 的路径**（勘察结论）必须在合同中标成 MVP 明示支持列表或明示不支持，避免执行器默认全 App。
- AstrLink 侧栏「路由」当前弱化固定路由 UI，但 Control API/组件仍在；**不要**把 AstrLink 当前侧栏状态误当成「无多级路由需求」——本产品 V1（005）仍要做 priority/failover。

## Scope

In scope:
- `plans/001-gateway-centric-contract.md`（本文）
- `plans/README.md`（状态）
- `crates/domain/**`（合同类型、写回参数、测试夹具）
- `crates/store/**`（migration / schema / 空或薄 repository 桩）
- 必要时 `Cargo.toml` / workspace 成员注册（仅 schema crate 需要时）

Out of scope:
- `crates/cursor-gateway/**` 行为重写 — 属 `002`
- `crates/adapters-*/**` 全面改 write_live — 属 `003`（本 plan 仅契约测试所需的最小钩子可动）
- `crates/session/**` 编排重写 — 属 `004`
- `crates/ui/**` 信息架构 — 属 `004`/`005`
- Cursor MITM、隐私、auto、订阅 OAuth、Gemini/RelayKit — 延后
- 任何 AstrLink 仓库文件

## Commands

| Purpose | Command | Expected result |
| --- | --- | --- |
| Domain check | `cargo check -p domain` | exit 0 |
| Store check | `cargo check -p store` | exit 0 |
| Contract tests | `cargo test -p domain`（及本 plan 引入的 adapter 契约测试包） | exit 0 |
| Workspace compile (acceptance) | `cargo check --workspace` | exit 0 |

## Done criteria

- [ ] All listed commands pass（含 acceptance）。
- [ ] 访问令牌、网关设置、最小 ledger 的 schema/类型已冻结且有 migration 或等价物。
- [ ] 至少两个 App 的「指向网关」写回契约测试存在且会因写上游而失败。
- [ ] Decisions & tradeoffs 全部被后续 plan 引用时无歧义（未知令牌拒绝；always-on；DB 存上游 / live 存网关）。
- [ ] No out-of-scope files changed.
- [ ] `plans/README.md` 中 001 状态更新。

## STOP conditions

- A fact cited under Decisions & tradeoffs no longer holds.
- The outcome requires out-of-scope files.
- A validation command fails twice after one reasonable fix.
- 执行器试图在本 plan 实现完整网关监听或 UI。

## Maintenance notes

合同变更必须先改 001 再改 002/003；禁止平行 plan「顺手改列」。端口默认值一旦写入用户数据，变更需迁移说明。

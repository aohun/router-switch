# Plan 005: 多级路由 + 请求账本查询（V1）

> This plan is an outcome contract, not a step-by-step script. Understand the
> requirement and the recorded decisions, then design the implementation
> yourself against the live code. Run milestone validations as you go only if
> you are also the verifier — a delegated executor implements only, and
> verification happens outside its session. Stop on any STOP condition. When
> complete, update this plan in `plans/README.md`.
>
> Drift check: `git diff --stat 7e840ea..HEAD -- crates/cursor-gateway crates/store crates/domain crates/ui plans/004-mvp-integration.md plans/005-routing-and-request-ledger.md`

## Status

- Priority: P2
- Effort: L
- Risk: HIGH
- Depends on: plans/004-mvp-integration.md
- Category: feature
- Planned at: `7e840ea`, 2026-09-26

## Requirement

在 MVP 网关闭环之上，补齐 AstrLink 级**多级智能路由的核心子集**与**请求记录可查**：模型别名/重定向、多 upstream target + priority、失败重试/failover、以及基于网关 ledger 的请求列表（含上游尝试结果）。不包含隐私、auto 分类、订阅 OAuth。

## Decisions & tradeoffs

- **路由在网关内执行，不在 adapters 内**: CLI 只看见网关模型名；选上游是网关职责。Rejected: 继续靠改各 CLI 模型映射冒充路由。
  Based on: 方案 A；AstrLink 路由在 ingress/core。

- **先做固定多 target + failure policy，不做 astrlink/auto**: 分类 worker 仍延后。Rejected: V1 绑本地分类模型。
  Based on: 方案 A 可选后期；AstrLink auto 为 XL。

- **Ledger 查询 UI 基于 002 写入的账本**: 可扩展列，但不得改回「只扫 CLI 会话文件」作为唯一来源。
  Based on: 002/004 决策。

- **行为参考 AstrLink，不移植 Go**: 失败动作语义可对照 AstrLink `failure-policy-model.ts`（retry / retry_and_failover 等），在 Rust 中自立模型。
  Based on: `/Users/wayne/Desktop/git/AstrLink/apps/desktop/src/failure-policy-model.ts`（产品参考，非依赖）。

## Direction

### Milestone 1: 路由配置模型与存储

可持久化：model redirect/alias、route → ordered targets（provider_id + model + priority）、default failure policy。  
Validation: `cargo test -p store -p domain` -> exit 0。

### Milestone 2: 网关执行路由与 failover

请求按路由表选 target；失败时按策略重试/切换 target；ledger 记录每次 attempt。  
Validation: `cargo test -p cursor-gateway` -> exit 0（含 failover 单测）。

### Milestone 3: UI 路由与请求列表

用户可编辑路由/失败策略，并浏览请求账本列表与基础详情。  
Validation: `cargo check -p ui`；`cargo test --workspace` -> exit 0。

## Landmines

- MVP 的「令牌绑定默认 provider」必须在引入路由表后有明确覆盖规则（路由命中优先 vs 默认），写进实现与测试，避免双策略并存无文档。
- 会话粘性（AstrLink channel stickiness）**本 plan 可选**：若做，单独立项测试；不做则勿留半残字段。
- 转发身份（订阅 UA）在无订阅 OAuth 时影响面小；若仍转发第三方兼容网关，保持不注入产品名到上游（对齐 AstrLink AGENTS 精神）。

## Scope

In scope:
- `crates/domain/**`（路由/失败策略模型）
- `crates/store/**`（路由表、ledger 查询）
- `crates/cursor-gateway/**`（执行路由）
- `crates/ui/**`（路由页、请求列表）
- `plans/README.md` 状态

Out of scope:
- 隐私、auto 分类、订阅 OAuth、Gemini/RelayKit、Cursor MITM 增强
- adapters 再改写（除非路由模型名需要同步文档化）

## Commands

| Purpose | Command | Expected result |
| --- | --- | --- |
| Domain/store tests | `cargo test -p domain -p store` | exit 0 |
| Gateway tests | `cargo test -p cursor-gateway` | exit 0 |
| Workspace tests | `cargo test --workspace` | exit 0 |
| UI routing smoke (acceptance) | 配置两条 priority target，制造上游失败，确认 failover 与 ledger attempt | 行为符合 Requirement |

## Done criteria

- [ ] Automated commands pass.
- [ ] 多 target + failover 有失败会切换的自动化证明。
- [ ] 请求列表可读 ledger（非仅 tokens-core）。
- [ ] 未实现 auto/隐私/订阅 OAuth。
- [ ] No out-of-scope files changed.
- [ ] `plans/README.md` 状态更新。

## STOP conditions

- 004 未 DONE。
- 路由与「令牌默认 provider」优先级无法说清。
- 两次修复后网关路由测试仍失败。

## Maintenance notes

后续若做 stickiness / auto / 隐私，应新开 plan，并先扩展 001 风格的合同，而不是在 005 上无限堆叠。

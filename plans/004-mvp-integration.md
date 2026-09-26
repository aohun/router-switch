# Plan 004: MVP 集成 — Session 编排与一键指向网关

> This plan is an outcome contract, not a step-by-step script. Understand the
> requirement and the recorded decisions, then design the implementation
> yourself against the live code. Run milestone validations as you go only if
> you are also the verifier — a delegated executor implements only, and
> verification happens outside its session. Stop on any STOP condition. When
> complete, update this plan in `plans/README.md`.
>
> Drift check: `git diff --stat 7e840ea..HEAD -- crates/session crates/ui crates/app crates/store crates/cursor-gateway plans/002-always-on-gateway-and-tokens.md plans/003-adapters-point-to-gateway.md plans/004-mvp-integration.md`

## Status

- Priority: P1
- Effort: L
- Risk: HIGH
- Depends on: plans/002-always-on-gateway-and-tokens.md, plans/003-adapters-point-to-gateway.md
- Category: feature
- Planned at: `7e840ea`, 2026-09-26

## Requirement

把 002 的网关与 003 的适配器接到同一产品闭环：**启动/恢复时网关 always-on**；用户配置 API Key 类提供商、创建访问令牌后，可对选中 App **一键写入网关 Base URL + 令牌**；重启后 live 与网关端口仍一致。UI 信息架构至少能完成该路径（不必一次做完美导航重构）。

## Decisions & tradeoffs

- **Session 成为「注册上游 + 可选写 CLI 指针」编排器**: 取代「切换服务商 = 把上游写入 live」为主路径。Rejected: 保留 `routing_plan` 仅在方言不匹配时写 compat（`session/src/lib.rs:2017`）。
  Based on: 001/方案 A；现状按需 compat 与网关中心叙事冲突。

- **单一默认网关端口来自 store settings**: UI/session/adapters 都读同一来源；随机端口 fallback 必须写回 store。Rejected: 各组件各自猜 `8787`。
  Based on: `COMPAT_DEFAULT_PORT`（`compat/mod.rs:37`）与忙时随机端口行为。

- **UI 最小可用优先于视觉重做**: 需要「提供商（API Key）」「访问令牌」「一键应用到 App」「网关状态/复制 Base URL」。Rejected: MVP 先做完整 AstrLink 级侧栏。
  Based on: 方案 A MVP 切片。

- **tokens-core 用量扫描保留但标为补充**: 网关 ledger 为权威流量账本；会话文件扫描可继续显示，但不得替代 ledger。Rejected: 删除 tokens-core。

## Direction

### Milestone 1: Session 启动网关 + 恢复指针

应用/工作区启动时启动网关；`restore_*` 逻辑按「经网关」重写 live，而不是按旧 compat 条件。  
Validation: `cargo test -p session` -> exit 0。

### Milestone 2: 一键应用 API

存在 session 级 API：`apply_gateway_pointer(app, token_id)`（名可不同）调用 003 写回并持久化「该 App 经网关」状态。  
Validation: `cargo test -p session` 含 apply/restore 断言 -> exit 0。

### Milestone 3: UI 最小路径

用户可在 UI 完成：添加 Key 提供商 → 创建令牌 → 复制 Base URL → 应用到至少一个 App。  
Validation: `cargo check -p ui -p app` -> exit 0；手工/acceptance 见 Commands。

## Landmines

- `write_live` 巨石（`session/src/lib.rs` 大量 `register_compat_target` 分支约 1735–1947）易漏改某一 App，导致「UI 显示已应用但 CLI 仍直连」。
- 与 Cursor MITM 同时改 `http.proxy` 时的设置冲突：经网关的通用路径不要误清/误写 Cursor proxy，除非用户在用 Cursor 旁路。
- 令牌吊销后 live 仍持旧令牌：MVP 至少在文档/UI 提示需重新应用；可做自动重写但不阻塞本 plan。

## Scope

In scope:
- `crates/session/**`
- `crates/ui/**`（完成最小路径所需）
- `crates/app/**`（启动钩子）
- `crates/store/**`（若需「app→gateway mode」状态字段且 001 已预留；未预留则先回 001 修合同）
- 为接线所需的 `crates/cursor-gateway/**` 薄封装调用
- `plans/README.md` 状态

Out of scope:
- 多 target 路由 / failover / 完整请求记录页 — 005
- 订阅 OAuth 进网关、隐私、auto、MITM 增强
- AstrLink 代码

## Commands

| Purpose | Command | Expected result |
| --- | --- | --- |
| Session tests | `cargo test -p session` | exit 0 |
| UI/app check | `cargo check -p ui -p app` | exit 0 |
| Workspace tests | `cargo test --workspace` | exit 0 |
| Manual smoke (acceptance) | 运行桌面应用：建提供商 → 建令牌 → 应用到 Codex 或 Claude → 用 CLI/curl 经网关打通一次 | 行为符合 Requirement |

## Done criteria

- [ ] Automated commands pass.
- [ ] 启动后网关可连；一键应用后 live 为网关指针。
- [ ] 重启后网关端口与 live 一致（或自动修复）。
- [ ] Acceptance smoke 通过。
- [ ] No out-of-scope files changed.
- [ ] `plans/README.md` 状态更新。

## STOP conditions

- 002 或 003 未 DONE。
- store 缺少 001 未定义字段却「顺手加列」而不更新 001。
- 两次修复后 session 测试仍失败。

## Maintenance notes

MVP 闭环完成后，对外文案应从「Provider Switcher」转向「Local AI Gateway」；旧「仅切换上游」路径若保留，须在 UI 上明确降级为高级/兼容模式。

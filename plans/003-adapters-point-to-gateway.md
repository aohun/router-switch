# Plan 003: Adapters 一律指向网关（写回语义）

> This plan is an outcome contract, not a step-by-step script. Understand the
> requirement and the recorded decisions, then design the implementation
> yourself against the live code. Run milestone validations as you go only if
> you are also the verifier — a delegated executor implements only, and
> verification happens outside its session. Stop on any STOP condition. When
> complete, update this plan in `plans/README.md`.
>
> Drift check: `git diff --stat 7e840ea..HEAD -- crates/adapters-codex crates/adapters-claude crates/adapters-claude-desktop crates/adapters-grok crates/adapters-opencode crates/adapters-pi crates/adapters-zcode crates/adapters-workbuddy crates/domain plans/001-gateway-centric-contract.md plans/003-adapters-point-to-gateway.md`

## Status

- Priority: P1
- Effort: M
- Risk: MED
- Depends on: plans/001-gateway-centric-contract.md
- Category: feature
- Planned at: `7e840ea`, 2026-09-26

## Requirement

当用户选择「经网关使用」时，各 AI CLI/客户端的 live 配置必须被写成**网关 Base URL + 本地访问令牌**，而不是上游 URL + 上游 Key。DB 内 providers 仍保存真实上游（供网关出站）。本 plan 交付适配器层与 domain 写回助手；**不**启动网关、**不**改 session 总控（004 接线）。

## Decisions & tradeoffs

- **写回内容严格按 001 合同**: live = 网关指针；store = 上游真相。Rejected: 双写上游到 live「顺便兼容」。
  Based on: `plans/001-gateway-centric-contract.md`。

- **MVP 明示支持的 App 列表** `(decided while planning)`: **必须**完成 Codex、Claude Code、Grok、OpenCode、Pi 的指向网关写回；Claude Desktop 若现有 mapping 路径依赖 compat 则一并改到网关令牌。**ZCode / WorkBuddy**：能做则做，做不到则在代码与测试中显式 `unsupported` 或跳过并文档化，禁止静默直连却显示「已接入网关」。
  Based on: 勘察结论 ZCode 可不经 compat；避免假成功。

- **官方订阅直连模式可保留为显式模式**: 「不经网关 / 官方」仍可写官方配置；与「经网关」互斥，不得混写上游 Key 到经网关模式的 live 文件。
  Based on: 现有 adapters 官方备份/还原模式（如 Codex `.official.bak` 一类行为）。

- **不在此 plan 修改 `session::Workspace::write_live` 巨石函数的控制流**: 只提供 adapters/domain 可调用的纯函数或清晰 API，供 004 调用。Rejected: 003 大改 session 导致与 002 合并冲突。
  Based on: 并行作用域分离；`session/src/lib.rs` 为共享编排面，属 004。

## Direction

### Milestone 1: Domain 写回助手

给定 gateway port、local token、原 base path 规则，产出各 App 可消费的「网关化 settings」补丁结构。  
Validation: `cargo test -p domain` -> exit 0（覆盖 URL 拼接与「不会把上游 key 放进输出」）。

### Milestone 2: 必修 App 适配器

Codex / Claude / Grok / OpenCode / Pi（+ 可行的 Claude Desktop）在「经网关」输入下写入网关字段；有回归测试。  
Validation: `cargo test -p adapters-codex && cargo test -p adapters-claude && cargo test -p adapters-grok && cargo test -p adapters-opencode && cargo test -p adapters-pi` -> exit 0。

### Milestone 3: 非支持 App 的明确行为

ZCode/WorkBuddy 等要么实现，要么返回明确错误/no-op 策略并有测试锁定。  
Validation: 相关 `cargo test -p adapters-zcode` / `adapters-workbuddy`（若保留）-> exit 0。

## Landmines

- Claude 使用 env 键 `ANTHROPIC_BASE_URL` / `ANTHROPIC_API_KEY` 等（`adapters-claude`）；改写时勿漏 AUTH_TOKEN 变体。
- Codex 同时动 `config.toml` 与 `auth.json`；只改一处会导致 CLI 仍用旧凭据。
- OpenCode / Pi 的 JSON 字段名与嵌套 provider 对象易写错键；以现有 adapter 测试为锚。
- 001 契约测试若已存在，本 plan 应使其从「红」变「绿」，而不是删除契约。

## Scope

In scope:
- `crates/domain/**`（写回助手；扩展 001 契约）
- `crates/adapters-codex/**`
- `crates/adapters-claude/**`
- `crates/adapters-claude-desktop/**`
- `crates/adapters-grok/**`
- `crates/adapters-opencode/**`
- `crates/adapters-pi/**`
- `crates/adapters-zcode/**`（支持或显式不支持）
- `crates/adapters-workbuddy/**`（同上）
- `plans/README.md` 状态

Out of scope:
- `crates/cursor-gateway/**` — 002
- `crates/session/**` — 004
- `crates/ui/**`、`crates/app/**` — 004
- 路由策略、ledger UI — 005

## Commands

| Purpose | Command | Expected result |
| --- | --- | --- |
| Domain tests | `cargo test -p domain` | exit 0 |
| Adapter tests | `cargo test -p adapters-codex -p adapters-claude -p adapters-grok -p adapters-opencode -p adapters-pi` | exit 0 |
| Optional adapters | `cargo test -p adapters-zcode -p adapters-workbuddy -p adapters-claude-desktop` | exit 0 或按「显式跳过」策略通过 |
| Workspace check (acceptance) | `cargo check --workspace` | exit 0 |

## Done criteria

- [ ] All listed commands pass.
- [ ] 必修 App 在经网关模式下 live 字段为网关 URL + 本地令牌（测试断言）。
- [ ] 经网关模式 live 中不出现上游 API Key。
- [ ] 未支持 App 行为明确且测试锁定。
- [ ] No out-of-scope files changed.
- [ ] `plans/README.md` 状态更新。

## STOP conditions

- 001 未 DONE。
- 为图省事在 live 中继续写入上游 Key。
- 修改 `session` 编排导致与 002 范围冲突。

## Maintenance notes

新增 App 适配器时必须实现同一「PointToGateway」合同测试模板。

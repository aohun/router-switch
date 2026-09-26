# Plan 002: Always-on 网关 + 本地令牌鉴权 + 最小账本写入

> This plan is an outcome contract, not a step-by-step script. Understand the
> requirement and the recorded decisions, then design the implementation
> yourself against the live code. Run milestone validations as you go only if
> you are also the verifier — a delegated executor implements only, and
> verification happens outside its session. Stop on any STOP condition. When
> complete, update this plan in `plans/README.md`.
>
> Drift check: `git diff --stat 7e840ea..HEAD -- crates/cursor-gateway crates/store Cargo.toml plans/001-gateway-centric-contract.md plans/002-always-on-gateway-and-tokens.md`

## Status

- Priority: P1
- Effort: L
- Risk: HIGH
- Depends on: plans/001-gateway-centric-contract.md
- Category: feature
- Planned at: `7e840ea`, 2026-09-26

## Requirement

用户需要一个**始终可连**的本机推理入口：用本地访问令牌调用 OpenAI Chat / Anthropic Messages / OpenAI Responses（MVP 三种已有 compat 方言），网关将请求转发到 store 中配置的上游，并写入最小请求账本。完成后，任意 HTTP 客户端（不限于 RS adapters）只要持有令牌即可调用；未知令牌必须失败。

本 plan **不**改各 CLI live 文件（属 003），**不**重写 session 总编排（属 004）。

## Decisions & tradeoffs

- **遵守 001 全部身份/schema 合同**: 本地令牌鉴权；拒绝未知令牌；always-on API；ledger 最小列。Rejected: 继续 `resolve` 匹配上游 Key / first-wins。
  Based on: `plans/001-gateway-centric-contract.md` Decisions；`compat/mod.rs:103-113`。

- **从 CompatGateway 演进为 gateway-core，而非新建平行第二监听器**: 提升/重构现有 `crates/cursor-gateway/src/compat/**` 与 `provider/**` 成为 always-on 核心；避免两套 `:port` 打架。Rejected: 另起进程旁路 compat 导致双端口。
  Based on: compat 已具备三方言入站（`build_compat_router` 路由表）。

- **出站仍复用现有 provider 客户端抽象**: Chat / Anthropic / Responses 出站走现有 `provider/*`。Rejected: MVP 重写全部 upstream HTTP。
  Based on: `crates/cursor-gateway/src/provider/mod.rs`。

- **提供商选择（MVP）**: 在多级路由（005）落地前，用**显式默认提供商**或「令牌绑定的默认 provider_id」选上游；不得假装已实现 priority 链。Rejected: 在本 plan 实现完整 FixedRoutes。
  Based on: 001 将多 target 路由划给 005。

- **与 Cursor MITM 解耦**: 不修改 MITM 拦截语义作为本 plan 交付；共享 crate 内文件变动须保持 MITM 可编译。Rejected: 把 MITM 并进通用网关路径。

## Direction

### Milestone 1: 令牌签发与鉴权中间件

可创建/列出/吊销访问令牌（store API）；HTTP 入站从 `Authorization: Bearer` 或 `x-api-key` 解析后**只**接受本地令牌。  
Validation: `cargo test -p store -- <tokens>` 与 `cargo test -p cursor-gateway -- <auth 或 gateway 过滤器>` -> exit 0（含「未知令牌 → 401/403」）。

### Milestone 2: Always-on 监听与三方言转发

网关可在配置端口启动；`/v1/chat/completions`、`/v1/messages`、`/v1/responses`、`/v1/models`（及现有无前缀别名若保留）在有效令牌下转发到上游默认提供商。  
Validation: `cargo test -p cursor-gateway -- <compat/gateway 集成或单元过滤器>` -> exit 0。

### Milestone 3: 最小 request ledger 写入

每次（或每次完成的）推理尝试写入 001 定义的 ledger 行；失败路径也有记录。  
Validation: `cargo test -p cursor-gateway` 或 `cargo test -p store` 中断言 ledger 行存在 -> exit 0。

## Landmines

- 旧行为「Bearer = 上游 Key」可能被现有测试依赖；改鉴权后必须更新测试，禁止保留 first-wins 回退。
- 端口占用时现状会落到随机端口（`compat/mod.rs` start 逻辑）；产品要可配置，随机端口仅作 fallback 且必须写回 settings，否则 adapters 会写错地址（004 集成时再强约束，本 plan 至少把实际 port 暴露给调用方）。
- `tokens-core` 扫描的是 CLI 会话文件，**不是**本 ledger；勿复用错系统。

## Scope

In scope:
- `crates/cursor-gateway/**`（compat/provider/runtime 中与通用网关相关部分；MITM 仅允许为编译修复的最小改动）
- `crates/store/**`（tokens / ledger / gateway settings 的实现，遵守 001 schema）
- `Cargo.toml` / workspace 依赖（若拆 crate）
- `plans/README.md` 状态

Out of scope:
- `crates/adapters-*/**` — 003
- `crates/session/src/lib.rs` 总编排、`write_live` 语义翻转 — 004
- `crates/ui/**` — 004/005
- 多 target 路由 / failover — 005
- Cursor MITM 功能增强、隐私、订阅 OAuth

## Commands

| Purpose | Command | Expected result |
| --- | --- | --- |
| Gateway tests | `cargo test -p cursor-gateway` | exit 0 |
| Store tests | `cargo test -p store` | exit 0 |
| Workspace check (acceptance) | `cargo check --workspace` | exit 0 |

## Done criteria

- [ ] All listed commands pass.
- [ ] 未知令牌无法调用推理接口；有效令牌可走通至少 Chat 与 Messages 之一的自动化测试。
- [ ] 网关可 always-on 启动并暴露实际端口。
- [ ] Ledger 有自动化断言。
- [ ] 未引入上游-Key-as-Bearer 的选路回退。
- [ ] No out-of-scope files changed.
- [ ] `plans/README.md` 状态更新。

## STOP conditions

- 001 未 DONE 或 schema 与实现冲突。
- 两次修复后测试仍失败。
- 为通过测试恢复 first-wins 或上游 Key 鉴权。

## Maintenance notes

令牌明文只在创建时返回一次（若采用 hashed-at-rest，在 001 类型中已约定则必须遵守）。

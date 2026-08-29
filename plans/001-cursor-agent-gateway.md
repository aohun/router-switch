# Plan 001: Rewrite Cursor Agent gateway from scratch

> This plan is an outcome contract, not a step-by-step script. Understand the
> requirement and the recorded decisions, then design the implementation
> yourself against the live code. Run milestone validations as you go only if
> you are also the verifier — a delegated executor implements only, and
> verification happens outside its session. Stop on any STOP condition. When
> complete, update this plan in `plans/README.md`.
>
> Drift check: `git diff --stat df08d1e..HEAD -- crates/cursor-gateway crates/session/src/lib.rs crates/domain/src/cursor.rs crates/ui/locales crates/ui/src/app_view.rs Cargo.toml crates/cursor-gateway/Cargo.toml`

## Status

- Priority: P1
- Effort: L
- Risk: HIGH
- Depends on: none
- Category: feature
- Execution: subagent
- Planned at: `df08d1e`, 2026-08-29

## Requirement

Router Switch already has Cursor as an app kind (服务商表单、官方/第三方、网关启停 UI). The current `crates/cursor-gateway` is a thin, incomplete stub: MITM + fake Ultra + model list + prompt-to-text streaming. It cannot run Cursor Agent (no exec protocol, no tool resume, proto drops client tool results).

**Rewrite `crates/cursor-gateway` from scratch.** Do not extend, wrap, or keep compatibility with the old modules (`proto.rs` subset, `session.rs::start_agent_run`, prompt-only `bidi_append`, dummy `RunSSE` subscriber). Treat the crate as empty design space, ported from cursor-byok's Agent kernel.

Once this is done, with the gateway running and a **third-party** Cursor 服务商 selected:

- Cursor Agent / Chat keep using the official Cursor UI and client-side tools.
- Model calls go to that 服务商's OpenAI- or Anthropic-compatible API.
- Tool calls round-trip through Cursor's Agent protocol (`exec_server_message` out, `exec_client_message` / interaction responses back) and resume the model.
- An **official** Cursor 服务商 does not send Agent traffic into the local OpenAI adapter; it is forwarded to Cursor upstream while the gateway is up.
- Workspace can still start/stop the gateway and show CA / port status in the existing Cursor page.

Adjacent wrong solutions: patching Cursor.app (CCursor); running Node inside Cursor; keeping the old prompt-only session loop "for now"; vendoring cursor-byok's Tauri desktop / ads / semble embedder; only polishing the 服务商 UI.

## Decisions & tradeoffs

- **Greenfield rewrite of `crates/cursor-gateway`. (review)** Delete and replace the crate internals. Rejected: incremental salvage of current `runtime.rs` / `session.rs` / hand-written `proto.rs` / `start_agent_run`. Those files are not a compatibility contract. Based on: user review of this plan, 2026-08-29.

- **Reference implementation is cursor-byok, not CCursor.** Port MITM + local Agent kernel from `/Users/wayne/Desktop/git/cursor-byok/server/src/cursor/` and `server/src/run/`. Rejected: CCursor installer that rewrites `workbench.js` / `cursor-agent-host` / extension-host signatures. CCursor `tools.ts` may be read as protocol notes only.

- **Keep the Router Switch product seams, not the old gateway types.** Stay a workspace crate started from `Workspace`, with Cursor 服务商 in `domain` + store + GPUI page. Rejected: a new desktop app, a VS Code extension, or a second crate name. The old `CursorGatewayRuntime` method set may be replaced; session/UI glue must be rewritten to the new API rather than preserving `set_settings` / `GatewayError` shapes. Based on: `crates/session/src/lib.rs:1016`, `crates/ui/src/app_view.rs:2532`.

- **Required runtime behaviors after rewrite (not inherited code):** on start, MITM only `*.cursor.sh`, inject Cursor `http.proxy` + disable HTTP/2 + system certs, manage a local CA, optionally inject a local Ultra stub into `state.vscdb` if no access token exists; on stop, clear those proxy keys. Implement these by porting cursor-byok harness, not by keeping `crates/cursor-gateway/src/harness/*` line-for-line.

- **Third-party = local Agent; Official = upstream passthrough.** When `CursorSettings.kind` is `ThirdParty`, `BidiAppend` + `RunSSE` run the local kernel. When `Official`, those paths (and unimplemented Agent RPCs) forward to `api2.cursor.sh`. Rejected: treating official `base_url: https://api2.cursor.sh` + `provider_type: "official"` as an OpenAI Chat adapter (`crates/domain/src/cursor.rs:243`).

- **Tools execute in the Cursor client, not in Router Switch.** Emit `exec_server_message` / interaction queries on SSE, wait for Bidi results, resume the model. Server-side exceptions: WebSearch / WebFetch. Rejected: in-process Shell/Read/Grep/Write.

- **Generate `agent.v1` from vendored proto.** Vendor `agent_v1.proto` from cursor-byok and compile with `prost-build` (see cursor-byok `server/build.rs`). Keep a **hand-written aiserver subset** for Bidi / ErrorDetails / catalog; do **not** compile full `aiserver_v1.proto` (duplicate message names). Rejected: any reuse of today's `AgentClientMessage { run_request }` struct.

- **Bidi `append_seqno` is ordered.** Apply client appends in sequence. Rejected: process-each-append-on-arrival.

- **Conversation/tool-round state lives in the new gateway, projected to provider messages.** Do not persist Agent transcripts into `crates/store` (`app.db` remains 服务商 SSOT). Rejected: CCursor "client checkpoint is source of truth"; rejected: merging cursor-byok's run/message tables into `app.db`.

- **Blob / request-context fetch is in scope.** If the client sends blob ids, kv-get them before the first provider pass. Rejected: a kernel that only reads `user_message.text`.

- **SemanticSearch is a degraded fallback, not semble.** Grep/glob-style client exec or a short tool error. Rejected: `crates/semble-core` / ONNX.

- **Do not add Gemini, Tab/Cpp, or cursor-byok desktop/metrics.** Provider types: `openai-chat` / `openai-responses` / `anthropic`. Other Cursor RPCs fall through the upstream proxy. Rejected: CCursor's 27-service Fastify surface.

- **Port prompts/tool catalog from cursor-byok.** Copy/adapt `cursor-byok/server/prompt/cursor/` into the new crate assets. Rejected: homemade tool names Cursor cannot exec.

- **Provider streams must emit tool_use.** Anthropic and both OpenAI adapters emit start/args-delta/end tool events from real tool_use / tool_calls / Responses function calls. Extra params must not overwrite `model` / `stream` / `messages` / `tools` / `system`.

- **Tests and docs use mock keys and example hosts only.** `sk-mock-key-12345`, `https://api.example.com/v1`. Never copy live keys from `~/.ccursor`, cursor-byok data dirs, or this machine's Cursor DB. Based on: `AGENTS.md` §1.

- **UI chrome stays; copy may change.** Cursor 服务商 list + gateway banner remain. Banner must describe Agent BYOK, not "transparent forward" only. Both locale files. Rejected: cloning cursor-byok's dashboard.

- **Workspace deps stay Rust.** prost-build / protoc-bin-vendored / hudsucker / axum on `cursor-gateway`. No Node toolchain.

## Direction

Replace `crates/cursor-gateway/src/**` with a cursor-byok-shaped kernel. Old files are not a starting point; after the rewrite, a reader should not find `start_agent_run` or a three-field `AgentClientMessage`.

```text
Cursor IDE
  → settings.json http.proxy
  → MITM 127.0.0.1 (only *.cursor.sh)
  → Axum backend
       ├ Official kind → upstream api2.cursor.sh
       └ ThirdParty kind
            BidiAppend (ordered) ─┐
            RunSSE ◄──────────────┤ Agent actor
                                  │ prompt compile + tools
                                  │ provider stream
                                  │ exec_server_message
                                  │ wait exec_client_message
                                  ▼
                            user OpenAI/Anthropic API
```

Public seam the rest of the app needs (names may change):

- construct / start / stop gateway
- apply current `CursorSettings` without restarting the desktop process
- running? proxy port, backend port, CA trusted?, CA install command

`crates/session` and the Cursor banner must be rewired to that seam. Do not keep the old type names for their own sake.

Reference tree: `/Users/wayne/Desktop/git/cursor-byok/server/src/cursor/` and `server/src/run/`. Do not copy `apps/`, `crates/semble-core`, ads, or tab completion.

### Milestone 1: Empty-crate skeleton that boots

New module layout, generated `agent.v1`, MITM + Axum process, harness behaviors above, and a session-facing runtime API. Old `lib.rs` tests that encode the stub catalog/proto are gone and replaced. Official vs third-party routing exists even if the Agent loop is still a stub that returns a clean protocol error.

Validation: `cargo test -p cursor-gateway` → exit 0, including decode of a fixture `exec_client_message` (must not be dropped).

### Milestone 2: Local Agent actor + ordered Bidi + RunSSE

Third-party runs attach a session actor; Official `BidiAppend`/`RunSSE` proxy upstream. SSE subscriber and first append cannot miss each other (`request_id` rendezvous). Switching the current Cursor 服务商 updates live routing.

Validation: `cargo test -p cursor-gateway` route/session tests with a fake upstream → exit 0.

### Milestone 3: Tool round then model resume

Provider `tool_use` → Cursor exec/interaction frames on `RunSSE` → wait for seqno-ordered client result → feed model → continue until `Done` without tools. Cover Read or Shell plus one server-side WebFetch/WebSearch. Task/MCP dispatch to the client, not in-process.

Validation: `cargo test -p cursor-gateway` tool-loop integration test (fake provider + fake client appends) → exit 0.

### Milestone 4: Prompt compile + history/blob projection

First provider pass includes system/runtime prompts, tool catalog, prior turns, and blob-resolved extra context. Empty `user_message.text` with only blob/history refs must not become an empty-user-only prompt.

Validation: prompt/projection tests in `cargo test -p cursor-gateway` → exit 0.

### Milestone 5: App glue, i18n, workspace gates

Session + Cursor banner talk to the new runtime. Locales updated. `cargo test --all` and `cargo fmt --check` pass. No live API keys in the diff.

Validation: `cargo test --all` → exit 0; `cargo fmt --check` → exit 0.

## Landmines

- **Do not "leave the old loop as fallback".** The stub `start_agent_run` plus `_ => {}` on tool events is the bug. A rewrite that still calls it is not done.

- **Official `provider_type` is `"official"`.** A generic OpenAI default for unknown types will POST chat/completions at `api2.cursor.sh` (`crates/domain/src/cursor.rs:249`). Routing must key off `CursorKind`, not adapter sniffing.

- **aiserver proto is not a drop-in compile.** cursor-byok `server/build.rs` — full `aiserver_v1.proto` generates invalid Rust. Generate `agent.v1` only; hand-write Bidi/ErrorDetails/catalog.

- **RunSSE vs first BidiAppend race.** Cursor opens SSE by `request_id` independently of the first append. The actor must rendezvous on `request_id`; a dummy disconnected receiver is a silent hang.

- **HTTP/2 must stay disabled in injected settings.** Cursor 3.13+ Agent Host can bypass MITM via WebSocket/HTTP2. MITM depends on HTTP/1.1 + `*.cursor.sh` allowlist.

- **Worktree is already dirty** with unrelated updater / tokens-core / usage_service. Do not touch those files even if they sit next to Cursor UI.

- **Account injection is a stub, not login.** Local Ultra email/token constants must remain fake. Do not copy real `state.vscdb` tokens into the repo or into tests.

- **`crates/session` re-exports `CaState` / `LoadedCa` / `CursorGatewayRuntime`.** A full type rename is allowed and expected; update session (and any UI calls) in the same change so the workspace still compiles.

## Scope

In scope:

- `crates/cursor-gateway/` in full (delete/replace sources, tests, `Cargo.toml`, `build.rs`, vendored proto, prompt assets)
- `crates/session/src/lib.rs` glue to the new runtime API
- `crates/ui/locales/zh-CN.yml` and `crates/ui/locales/en.yml`
- `crates/ui/src/app_view.rs` Cursor banner / start-stop against the new seam
- `crates/domain/src/cursor.rs` only if the new kernel needs extra settings fields (keep Official/ThirdParty + provider_type model)
- Root `Cargo.toml` only if a workspace dependency is required

Out of scope:

- CCursor installer / Cursor.app patches
- cursor-byok `apps/desktop`, ads, semble/embeddings, tab/cpp, capture scripts
- Gemini or new `provider_type` values beyond openai-chat / openai-responses / anthropic
- Other app adapters
- `crates/tokens-core`, updater, usage_service
- Persisting Agent transcripts into `crates/store` / `app.db`
- Claude/Codex/Grok live-file writers
- Real Cursor.app e2e

## Commands

| Purpose | Command | Expected result |
| --- | --- | --- |
| Gateway tests | `cargo test -p cursor-gateway` | exit 0 |
| Workspace tests | `cargo test --all` | exit 0 |
| Format | `cargo fmt --check` | exit 0 |

No `(acceptance)` GUI suite. Agent behavior is asserted with fake provider + fake Bidi client. Do not require a live Cursor install or a live LLM.

## Done criteria

- [ ] All listed commands pass.
- [ ] Old stub kernel is gone: no prompt-only `start_agent_run` path remains as the Agent implementation.
- [ ] Third-party Cursor Agent completes at least one client-executed tool round and resumes the model (tests, not a live LLM).
- [ ] Official Cursor 服务商 Agent traffic is proxied upstream, not locally adapted as OpenAI.
- [ ] `exec_client_message` (or equivalent) decodes instead of being dropped.
- [ ] WebSearch or WebFetch has a server-side path; SemanticSearch does not pull in semble.
- [ ] Tests/fixtures use mock keys and example hosts only.
- [ ] New user-visible strings exist in both locale files.
- [ ] Session/UI compile against the new runtime API.
- [ ] Implementation follows every entry in Decisions & tradeoffs.
- [ ] No out-of-scope files changed.
- [ ] `plans/README.md` status is updated.

## STOP conditions

- A fact cited under Decisions & tradeoffs no longer holds.
- The outcome requires patching Cursor.app, adding a Node installer, or vendoring semble/desktop.
- A validation command fails twice after one reasonable fix.
- Generated `agent.v1` types cannot express exec/tool/kv messages without compiling full `aiserver_v1.proto`.
- Real API keys or `state.vscdb` tokens appear in the diff.
- The rewrite keeps the old `start_agent_run` text-only loop as a fallback.

## Maintenance notes

- `agent_v1.proto` is the schema source. Do not regress to a hand-written AgentClient/Server subset.
- After a Cursor major, first failures are usually dropped oneof fields or Agent Host bypassing the proxy. Check `disableHttp2` and MITM allowlist before rewriting the kernel.
- Gateway start still mutates the user's Cursor `settings.json` and may inject a local Ultra stub. Stop must clear proxy keys.
- Domain `CursorSettings` remains the 服务商 contract with the rest of the app; do not fork a second provider config model inside the gateway.

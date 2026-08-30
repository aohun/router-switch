# Plan 002: Auto-routing gateway for cross-protocol providers

> Outcome contract, in the style of plan 001.

## Status

- Priority: P1
- Effort: M
- Risk: MEDIUM
- Depends on: 001
- Category: feature
- Planned at: `44ab374`, 2026-08-30

## Requirement

Align with cc-switch's "上游格式（需开启路由）" capability: when the user
switches an AI tool (Claude Code / Codex / Grok / OpenCode / Pi) onto a
服务商 whose 请求协议 (`RequestProtocol`) differs from that tool's native
dialect, Router Switch must automatically start a local routing gateway,
rewrite the tool's live config to point at it, and convert protocols in
flight — instead of writing a direct connection that cannot work.

Native dialects: Claude/ZCode → Anthropic Messages, Codex → Responses,
Grok/OpenCode/Pi → Chat Completions. ZCode already encodes the protocol in
its own `provider_kind` (no gateway needed); Cursor always goes through the
Agent gateway and already supports all three upstream protocols.

## Delivered

- `cursor-gateway::compat`: a lazily-started localhost listener exposing
  `/v1/messages` (Anthropic), `/v1/chat/completions` (OpenAI Chat) and
  `/v1/responses` (OpenAI Responses). Each request is parsed into the
  canonical `ModelInvocation`, forwarded through the existing upstream
  `Provider` adapters (Anthropic / OpenAI Chat / OpenAI Responses), and the
  `ModelEvent` stream is rendered back in the inbound dialect — streaming SSE
  and non-streaming JSON, text / thinking / tool calls / usage. Targets are
  registered per `AppKind` and resolved by the API key each tool presents.
  `/v1/models` lists routed models for tools that fetch model lists.
- `session::Workspace`: switching onto a provider whose `request_protocol`
  differs from the app's native dialect now registers a `CompatTarget` (real
  upstream URL / key / model / model mappings), auto-starts the gateway
  (preferred port 8787, random fallback) and rewrites only the live config's
  base URL to `http://127.0.0.1:<port>`; the stored DB settings keep the real
  upstream. Switching back to a native-protocol provider unregisters the
  target and restores the direct connection. `Workspace::open` re-applies
  routed live configs so the gateway is listening again after a restart.
- Tests: compat unit tests (parse / extra-param normalization / registry
  resolution) plus end-to-end tests through a mock OpenAI Chat upstream for
  all three inbound dialects; session tests covering the routed switch, the
  reopen restore path, and the switch-back to a native provider.

## Not in scope (follow-ups)

- UI surfacing of gateway port / routed apps (session exposes
  `compat_gateway_port`, `compat_routed_apps`, `compat_target_for_app`).
- Two simultaneously-routed apps sharing an empty API key cannot be told
  apart (first target wins).
- Responses-inbound fidelity for exotic Codex fields (`include`, reasoning
  effort knobs) is best-effort; cross-dialect sampling knobs are whitelisted.

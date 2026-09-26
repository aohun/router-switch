# Plan 004: Authentication center (ChatGPT / xAI OAuth)

> Outcome contract, in the style of plans 001-003.

## Status

- Priority: P2
- Effort: M
- Risk: MEDIUM
- Depends on: none
- Category: feature
- Planned at: `bcc050e`, 2026-08-30

## Requirement

Port cc-switch's 认证中心 (auth center): a settings 认证 tab between 通用 and
高级 with two cards — ChatGPT (Codex OAuth) and xAI (Grok OAuth) — each showing
authentication status and a device-flow login button.

## Delivered

- `session::oauth`: OAuth 2.0 Device Authorization Grant clients ported from
  cc-switch — Codex (auth.openai.com deviceauth usercode/token endpoints,
  client `app_EMoamEEZ73f0CkXaXp7hrann`, verification URL
  `auth.openai.com/codex/device`, authorization_code + code_verifier exchange)
  and xAI (auth.x.ai discovery + standard device grant, client
  `b1a00492-073a-47ea-816f-4c329264a828`). JWT claims parsed (unverified) for
  account/email display.
- `session::auth_native`: credential persistence — Codex tokens are written to
  the native `~/.codex/auth.json` (`auth_mode: chatgpt` + tokens + last_refresh,
  same shape as Codex CLI), so the CLI itself becomes logged in; status reads
  that file (official `codex login` state is equally recognized). xAI tokens
  are stored in the app database (kv).
- `Workspace` API: `oauth_status` / `oauth_start_login` / `oauth_poll_once`
  (free fn, background-pollable) / `oauth_complete` / `oauth_logout`.
- UI: new `SettingsTab::Auth` (search keywords 认证/auth/oauth/chatgpt/xai/grok)
  with two cards per the screenshot: icon + title + subtitle, status row with
  已认证/未认证 tag and account email, and a full-width login button. Login
  opens the verification URL, surfaces the user code via toast, then runs a
  self-contained background poll loop (interval from the provider, deadline
  from expires_in) until Complete/Expired/Denied/Failed.
- Tests: JWT claim parsing, xAI/Codex roundtrip status persistence (native
  auth.json shape + logout), plus the existing i18n/unit suites.

## Not in scope (follow-ups)

- Multi-account management (cc-switch keeps an account list with default
  selection); router-switch logs in a single account per provider.
- Access-token auto refresh for status display; Codex CLI refreshes its own
  auth.json transparently.
- GitHub Copilot device flow card.

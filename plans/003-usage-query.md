# Plan 003: Provider usage (balance) query

> Outcome contract, in the style of plans 001/002.

## Status

- Priority: P2
- Effort: L
- Risk: MEDIUM
- Depends on: none
- Category: feature
- Planned at: `481ec9d`, 2026-08-30

## Requirement

Port cc-switch's per-provider 用量查询 (usage/balance query): each provider can
carry a scriptable query that fetches balance/quota from the vendor and shows
the result on the provider card, with optional auto refresh.

## Delivered

- `domain::usage_script`: `UsageScriptConfig` (enabled / template / code /
  timeout / auto interval / per-query api_key+base_url+NewAPI credentials),
  `UsageDataItem` / `UsageQueryResult` result models, and the three preset
  JS templates (自定义 / 通用模板 / NewAPI) ported from cc-switch.
- New `usage-query` crate: QuickJS (rquickjs) executor, ported 1:1 from
  cc-switch's `usage_script.rs` — `{{baseUrl}}/{{apiKey}}/{{accessToken}}/{{userId}}`
  substitution, memory/stack/interrupt-guarded runtime, HTTPS + same-origin
  enforcement (custom template exempt), 2–30s timeout clamp, result shape
  validation. Tests cover the security rules, an infinite-loop script, and an
  end-to-end roundtrip against a local mock upstream.
- `store`: `provider_usage_scripts` table (config + cached last result),
  CRUD + `list_usage_scripts` for refresh scheduling.
- `session`: save/get/delete usage script, `prepare_usage_query(_with_config)`
  (defaults resolved from the provider's own credentials via
  `extract_provider_probe_target`), `complete_usage_query` (parse + persist),
  blocking query for tests/background.
- UI: bar-chart entry button on every provider card, config page (enable
  toggle, template chips, optional credentials, timeout/interval, multi-line
  extractor editor, test-script / save / cancel), usage badge on provider
  cards, 60s auto-refresh loop for due providers, delete-provider cleanup.
- i18n: `usage_script.*` + `provider.usage_query` in zh-CN/en.

## Not in scope (follow-ups)

- cc-switch's dedicated Rust query paths: Token Plan (kimi/zhipu/minimax
  coding plans), official balance APIs (DeepSeek/StepFun/SiliconFlow/
  OpenRouter/Novita), GitHub Copilot — the script engine covers custom
  endpoints meanwhile.
- Script 格式化 (prettier) button.

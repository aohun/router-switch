# Plans

Execution order is top to bottom. Status: TODO, IN PROGRESS, DONE, BLOCKED.

| Plan | Title | Status | Priority | Effort | Depends on |
| --- | --- | --- | --- | --- | --- |
| [001](001-cursor-agent-gateway.md) | Rewrite Cursor Agent gateway from scratch | DONE | P1 | L | none |
| [002](002-auto-routing-gateway.md) | Auto-routing gateway for cross-protocol providers | DONE | P1 | M | 001 |

## Notes

- 001 is a single serial plan. A parallel split would share `crates/cursor-gateway` and session glue.
- Departure check: follow cursor-byok (MITM + local Agent kernel), not CCursor binary patches. Execution mode `subagent`. Review pause: do not execute until the user approves this plan.
- Review 2026-08-29: rewrite `crates/cursor-gateway` from scratch; do not salvage the stub kernel. Product seams (Cursor 服务商, Workspace start/stop, banner) stay and are rewired.

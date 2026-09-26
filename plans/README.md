# Plans — Router Switch 网关中心化

> 目标仓库：[aohun/router-switch](https://github.com/aohun/router-switch)  
> 基线勘察 SHA：`7e840ea`（2026-09-26）  
> 产品方向：**方案 A** — 以本机网关为中心；adapters 仅负责把各 AI CLI/客户端 live 配置改写为网关 `base_url` + 本地访问令牌。  
> 不整仓合并 AstrLink；按能力对照在 Rust/RS 侧重写。参考产品行为时可对照本地 AstrLink（`Calcium-Ion/AstrLink`）。

## 执行顺序

| 顺序 | Plan | 状态 | 依赖 | 说明 |
| --- | --- | --- | --- | --- |
| 1 | [001-gateway-centric-contract](./001-gateway-centric-contract.md) | TODO | — | **合同（串行）**：身份模型、SQLite 表、适配器写回语义、MVP 边界 |
| 2a | [002-always-on-gateway-and-tokens](./002-always-on-gateway-and-tokens.md) | TODO | 001 | **并行组**：always-on 网关 + 本地令牌鉴权 + 最小请求账本写入 |
| 2b | [003-adapters-point-to-gateway](./003-adapters-point-to-gateway.md) | TODO | 001 | **并行组**：adapters/domain 写回语义改为「一律指向网关」 |
| 3 | [004-mvp-integration](./004-mvp-integration.md) | TODO | 002, 003 | **串行集成**：session 编排、固定端口产品化、端到端闭环 |
| 4 | [005-routing-and-request-ledger](./005-routing-and-request-ledger.md) | TODO | 004 | **V1**：多 target 优先级路由、failover、请求记录查询面 |

并行组：`002` 与 `003` 在 `001` DONE 后可并发执行（作用域不相交；共享面冻结在 `001`）。

## 明确延后（本 roadmap 不建 plan）

- Cursor MITM / Agent BYOK（保持现有旁路，不作为通用网关底座）
- 本机隐私策略 + privacy-worker
- `astrlink/auto` 式分类选模 + classifier-worker
- 网关内 Codex/Claude/Grok **订阅 OAuth**（MVP/V1 不做；见 001 Decisions）
- Gemini 入站 / RelayKit 级协议转换（AGPL）
- AstrLink Agent 诊断 MCP/Skill
- 整仓合并 AstrLink（Go/Tauri）

## 里程碑产品叙事

```text
用户配置按量/兼容 API 提供商（一次）
    → 签发本地访问令牌
    → 网关 always-on（127.0.0.1:<port>）
    → 一键写回 CLI：base_url=网关, api_key=本地令牌
    → CLI 请求经网关转发上游（上游 Key 不出网关）
```

MVP = 001→004；智能路由与完整请求记录 UI = 005。

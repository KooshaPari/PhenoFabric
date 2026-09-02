# Integration Matrix

| Product | Consumes | Produces | Protocol | Failure behavior |
|---|---|---|---|---|
| AGSLAG | usage/cost/evidence summaries | budgets, priorities, stop/continue policy | events + query API | Fabric continues already-authorized local work within cached policy |
| AgilePlus | evidence/run refs | authorized WPs, requirements, gates | gRPC/HTTP/MCP adapter + events | new gated work pauses; running work follows lease |
| thegent | placement/run/surface status | TaskSpecs and realm requests | gRPC/HTTP/CLI adapter + events | Fabric continues accepted tasks; no new labor plan |
| Tracera | evidence refs and facts | accepted evidence/impact queries | events + API | local evidence spools and later reconciles |
| SessionLedger | session/run bundles | replay/context refs | events/artifact API | local spool; no runtime dependency |
| ShareCLI | selected host/realm execution and policy | process declarations, pressure, coalescing observations | local IPC/CLI/HTTP adapter | ShareCLI remains a local supervisor |
| NVMS | compiled execution/realm requests if retained | resource/realm adapter facts | library/gRPC adapter | affected realm providers unavailable |
| Ledgers | facts/usage/references | asset/research/repo context | events/query APIs | runtime continues with cached/noncritical context |

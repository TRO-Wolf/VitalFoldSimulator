# `vital-fold-engine/src/handlers/` — map

**Purpose:** HTTP endpoint logic. Each file is an actix-web handler module; `simulation.rs` is the large one driving the populate/simulate/reset lifecycle.

## Contents

| File | What it is |
|---|---|
| health.rs | `GET /health` → `{"status":"ok"}` |
| auth.rs | `POST /api/v1/auth/login` (DB) + `/admin-login` (env creds); issues JWTs |
| user.rs | `GET /api/v1/me` — profile from JWT claims |
| simulation.rs | All populate/simulate/timelapse/reset/visitors endpoints + `POST /admin/init-db`; spawns background tasks, tracks progress in `SimulatorState` |
| mod.rs | Module re-exports |

## I want to…

| I want to… | Go to |
|---|---|
| Add an endpoint | New fn here + register in [../routes.rs](../routes.rs) + document in [../../API.md](../../API.md) |
| Change populate/sync orchestration | simulation.rs |
| Change login / token issuance | auth.rs |
| Change schema-init behavior | simulation.rs → `init_database` |

## Pointers

- Parent: [src/](../map.md).
- Endpoint reference: [../../API.md](../../API.md). Auth middleware: [../middleware/](../middleware/map.md).

## Debug

- Overlapping populate/reset jobs are blocked by the `running` flag in [../engine_state.rs](../engine_state.rs) — a stuck flag returns 409-style "already running".
- Long ops are fire-and-poll: handler returns early, work continues in a spawned task; poll `GET /simulate/status`.
- New endpoints must be added to the OpenAPI derive so they show in Swagger UI.

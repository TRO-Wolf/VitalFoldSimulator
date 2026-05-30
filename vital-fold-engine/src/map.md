# `vital-fold-engine/src/` — map

**Purpose:** All Rust source for the service. Top-level files are the bootstrap + cross-cutting infrastructure; subdirectories hold the domain logic (handlers, generators, models) and supporting layers (db, middleware).

## Contents

| File | What it is |
|---|---|
| main.rs | Server bootstrap: config load, DSQL pool, OpenAPI, route binding, startup state hydration |
| config.rs | Env-var loading via `dotenvy` (`Config` struct) |
| routes.rs | Route registration — maps every path to its handler + JWT scope |
| engine_state.rs | Shared `SimulatorState` (`Arc` + atomics + `Mutex` sub-states); mutex poison recovery |
| errors.rs | `AppError` enum + `ResponseError` impl |
| [db/](db/map.md) | PgPool creation + IAM token refresh |
| [middleware/](middleware/map.md) | JWT bearer validation |
| [handlers/](handlers/map.md) | HTTP endpoint logic |
| [generators/](generators/map.md) | Synthetic data builders (one per domain) |
| [models/](models/map.md) | `sqlx::FromRow` structs for every table |

## I want to…

| I want to… | Go to |
|---|---|
| Find which handler serves a path | routes.rs → [handlers/](handlers/map.md) |
| Add an endpoint | [handlers/](handlers/map.md) + register in routes.rs |
| Change shared run state / counts | engine_state.rs |
| Add an env var | config.rs + `.env.example` |
| Change error → HTTP mapping | errors.rs |

## Pointers

- Parent: [crate root](../map.md).
- Architecture + request flow: [../ARCHITECTURE.md](../ARCHITECTURE.md).

## Debug

- Mutex access in `engine_state.rs` uses a `recover()` helper that logs-and-recovers on poison — never `.unwrap()` a lock (CLAUDE.md §7.1).
- `rand::ThreadRng` is `!Send`: in async generators, drop the RNG before `.await` (CLAUDE.md §7.5).
- Startup panics are intentional fail-fast (missing config / unreachable DB) in `main.rs`.

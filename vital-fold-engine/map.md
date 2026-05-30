# `vital-fold-engine/` — crate map

**Purpose:** The Rust crate — a single-binary `actix-web` service that generates synthetic healthcare data into Aurora DSQL + DynamoDB and serves an embedded Preact admin dashboard. Holds all source, the schema migration, the UI, and crate-level docs.

## Contents

| Entry | What it is |
|---|---|
| Cargo.toml / Cargo.lock | Dependencies + release profile (LTO, strip); lockfile checked in (binary crate) |
| .env.example | Copy-ready env template (DSQL endpoint, JWT secret, admin creds) |
| [API.md](API.md) | All 22 REST endpoints with curl examples |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Request flow, state, three-phase lifecycle, scaling |
| [DEVELOPMENT.md](DEVELOPMENT.md) | Local dev setup + code style |
| [INSTALLATION.md](INSTALLATION.md) | Aurora DSQL + DynamoDB + IAM + Docker + Render setup |
| [QUICKSTART.md](QUICKSTART.md) | 5-minute bootstrap (DSQL path) |
| [src/](src/map.md) | All Rust source |
| [migrations/](migrations/map.md) | `init.sql` DDL for all 16 tables |
| [static/](static/map.md) | Preact SPA served at `/` |

## I want to…

| I want to… | Go to |
|---|---|
| Build / run / test | `cargo build --release` · `cargo run --release` · `cargo test --all-targets` (from here) |
| Find an endpoint's handler | [API.md](API.md) → [src/handlers/](src/map.md) |
| Add or change a generator | [src/generators/](src/generators/map.md) |
| Change the schema | [migrations/init.sql](migrations/init.sql) + update [../docs/models-spec.md](../docs/models-spec.md) |
| Configure env vars | `.env.example` + [src/config.rs](src/config.rs) |

## Pointers

- Parent: [repo root](../map.md).
- Architecture overview: [ARCHITECTURE.md](ARCHITECTURE.md). Invariants: [../CLAUDE.md](../CLAUDE.md) §7.

## Debug

- Binds `0.0.0.0:8787` by default. Startup needs `DSQL_CLUSTER_ENDPOINT` + `JWT_SECRET` or it fails fast (see [src/config.rs](src/config.rs)).
- Schema not created yet → call `POST /admin/init-db` (runs `migrations/init.sql`).
- Release profile has `lto = true` + `codegen-units = 1`; release builds are slow — use debug for iteration.

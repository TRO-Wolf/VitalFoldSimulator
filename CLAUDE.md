# CLAUDE.md — VitalFold Engine Orientation

> Primary orientation document for Claude agents working in this repository.
> **Always skim this file first before taking any action.**

---

## 1. What this project is

**VitalFold Engine** is a Rust REST API that generates fully **synthetic** (fictitious) healthcare data for a fictional cardiac clinic and writes it into **Amazon Aurora DSQL** (PostgreSQL-compatible) plus **Amazon DynamoDB**. Its purpose is to give data-pipeline engineers a realistic but safe dataset to build against — including deliberately injected data-quality errors that stress-test ETL, identity-resolution, and billing workflows.

**Key facts (do not contradict):**
- All data is **fictitious**. The `fake` crate generates names/addresses/etc. Insurance carriers and clinic names are invented. There is **no PHI** anywhere in the repo.
- The README contains a prominent fictitious-data disclaimer — **never remove or weaken it**.
- CPT codes and RVU values are **approximate**, not clinically authoritative.
- This is a **single-binary service**, not a library. It serves HTTP + an embedded Preact admin dashboard.

---

## 2. Tech stack

**Language:** Rust, edition 2021, stable toolchain.

**Web framework:** `actix-web 4` (NOT axum). Related: `actix-web-httpauth`, `actix-files`, `tracing-actix-web`, `utoipa-actix-web`.

**Database layer:** `sqlx 0.8.6` (postgres, runtime-tokio-rustls, uuid, chrono, bigdecimal, macros, derive).

**AWS SDK v1:** `aws-config`, `aws-sdk-dsql`, `aws-sdk-dynamodb`, `aws-credential-types`.

**Auth:** `jsonwebtoken 9` (HS256) + `bcrypt 0.15`. Secrets come from env vars only.

**Synthetic data:** `fake = 4` (derive feature), `rand = 0.9`.

**OpenAPI / docs:** `utoipa 5` + `utoipa-swagger-ui 9`. Swagger UI is served at `/swagger-ui/`.

**Frontend:** Preact 10 + HTM 3 + Pico CSS, all loaded via CDN from [vital-fold-engine/static/](vital-fold-engine/static/). **No bundler, no build step, no `node_modules`** — do not introduce one without being asked.

**Release profile:** LTO on, `codegen-units = 1`, `strip = true` ([vital-fold-engine/Cargo.toml:47-51](vital-fold-engine/Cargo.toml#L47-L51)).

---

## 3. Repository layout

```
vitalFoldEngine/
├── README.md                    Disclaimer, quick start, data model overview
├── CLAUDE.md                    This file — agent orientation
├── CHANGELOG.md                 Keep-a-Changelog format; update under [Unreleased]
├── CONTRIBUTING.md              Code style + no-unwrap policy (see §7)
├── LICENSE                      MIT
├── .github/workflows/           CI: cargo check / test / clippy / fmt
├── docs/
│   ├── models-spec.md           Rust struct definitions for every table
│   ├── dynamo.md                DynamoDB schema & write strategy
│   ├── frontend.md              Preact dashboard architecture
│   ├── airflow-integration.md   Example DAGs for orchestration
│   └── history/                 Archived historical notes (not load-bearing)
│       ├── BUILD_HISTORY.md     Initial build: fixes, validation, lessons
│       ├── DEPRECATION_HISTORY.md  Dependency pruning & deprecation fixes
│       └── project-origins.md   Background / motivation, original prompt
└── vital-fold-engine/           The Rust crate
    ├── Cargo.toml               Dependencies + release profile
    ├── API.md                   All 22 REST endpoints with curl examples
    ├── ARCHITECTURE.md          System design, phases, state, scaling
    ├── DEVELOPMENT.md           Local dev + code style
    ├── INSTALLATION.md          Aurora DSQL + DynamoDB setup, Docker, Render
    ├── QUICKSTART.md            5-minute bootstrap
    ├── migrations/init.sql      DDL for all 16 tables (run once per DB)
    ├── static/                  Preact SPA served at /
    └── src/
        ├── main.rs              Server bootstrap, OpenAPI, route binding
        ├── config.rs            Env var loading (dotenvy)
        ├── routes.rs            Route registration
        ├── engine_state.rs      Shared SimulatorState (Arc + RwLock / Mutex)
        ├── errors.rs            AppError enum + ResponseError impl
        ├── db/                  PgPool setup + query helpers
        ├── middleware/auth.rs   JWT bearer validation
        ├── handlers/            HTTP handlers: auth, health, user, simulation
        ├── generators/          Synthetic data builders (one per domain)
        │                        appointment, clinic, insurance, medical_record,
        │                        patient, provider, rvu, survey, visit
        └── models/              sqlx::FromRow + serde structs for every table
```

Every tracked directory also carries a `map.md` — see §3.1.

### 3.1 Directory `map.md` navigation — mandatory

**Every tracked directory carries a single `map.md`**, with no exceptions — including
container directories that hold only subdirectories (e.g. `.github/`), whose map is thin and
points down to its child maps. The *only* exclusions are version-control metadata (`.git/`) and
git-ignored / vendored trees (`.claude/`, `target/`, anything in `.gitignore`) — these are not
repo content and never get a `map.md`.

Each `map.md` has the same five parts:
- **Purpose** — one or two sentences on what lives here.
- **Contents** — table of the files/subdirs and what each is.
- **I want to… → Go to** — task-oriented routing table.
- **Pointers** — links to the parent map, relevant child maps, and related docs.
- **## Debug** — common failure modes for this directory and where to look.

**Before editing any file:** read the `map.md` of every directory your task will touch, and use
it to navigate.

**Update rule (hard requirement — same lockstep as "update the doc in the same commit"):**
whenever code or docs are created, changed, moved, or deleted, update that directory's `map.md`
in the **same change** so it never drifts:
- New directory of any kind → create its `map.md` in the same change.
- New / renamed / deleted file → update the directory's `map.md` **Contents** and
  **I want to…** rows.
- Behavior, entry point, or failure mode changed → update the relevant **Purpose** / row /
  **## Debug** section.

A change is not "done" (per the §10 checklist) until the touched directories' `map.md` files
reflect it. Code is still truth; if you find a stale `map.md`, correct it as part of your change.

---

## 4. Build, run, test

```bash
# From the crate directory
cd vital-fold-engine

cargo build --release
cargo run --release        # listens on 0.0.0.0:8787 by default
cargo test --all-targets   # 24 unit tests (mostly middleware::auth)
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings   # warnings are errors
cargo audit                # RustSec advisory scan (cargo install cargo-audit)
```

CI (see [.github/workflows/ci.yml](.github/workflows/ci.yml)) runs two jobs on every push and PR — **test** (`cargo fmt --check` → `cargo clippy -D warnings` → `cargo check` → `cargo test`) and **audit** (`cargo audit`). Clippy is gated strictly (`-D warnings`); the crate also sets `[lints] unsafe_code = "forbid"` in [vital-fold-engine/Cargo.toml](vital-fold-engine/Cargo.toml). `cargo audit` ignores `RUSTSEC-2023-0071` (rsa, via the unused sqlx MySQL driver — postgres-only, no fix exists). Keep it green.

---

## 5. Environment configuration

Config is loaded in [vital-fold-engine/src/config.rs](vital-fold-engine/src/config.rs) via `dotenvy` from `vital-fold-engine/.env`.

**Required:**
- `DSQL_CLUSTER_ENDPOINT` — Aurora DSQL hostname (e.g. `xxx.dsql.us-east-1.on.aws`)
- `JWT_SECRET` — minimum 32 characters; used for HS256 signing

**Optional (with defaults):**
- `HOST` (`0.0.0.0`), `PORT` (`8787`)
- `DSQL_REGION` (`us-east-1`), `DSQL_DB_NAME` (`postgres`), `DSQL_USER` (`admin`)
- `DB_POOL_SIZE` (`10`)
- `JWT_EXPIRY_HOURS` (`24`)
- `ADMIN_USERNAME`, `ADMIN_PASSWORD` — if set, enables `POST /api/v1/auth/admin-login`
- `RUST_LOG` (`info`)

See `vital-fold-engine/.env.example` for a copy-ready template. **Never commit a real `.env`.**

---

## 6. Architecture at a glance

### Request flow
```
HTTP → actix-web Router → JWT middleware → Handler → generator / sqlx / AWS SDK → JSON
```

### Three-phase data lifecycle
Phase 1 — **Static reference data** (`POST /populate/static`):
insurance companies, insurance plans, providers, clinics, clinic schedules, patients, emergency contacts, demographics, patient_insurance, cpt_code.

Phase 2 — **Dynamic clinical activity** (`POST /populate/dynamic`, 7 steps):
clinic schedules (first run only) → appointments → medical records → visits → vitals → surveys → appointment_cpt line items. Only appointments with `status = 'completed'` produce downstream rows; `no_show` (~1%) and `cancelled` (~9%) leave just the appointment row.

Phase 3 — **DynamoDB sync** (`POST /simulate/date-range` and friends):
read completed visits + vitals from Aurora for a time window and write denormalized copies into two DynamoDB tables with TTL.

### Shared state — [vital-fold-engine/src/engine_state.rs](vital-fold-engine/src/engine_state.rs)
`SimulatorState` is an `Arc<...>` handed to every handler via `web::Data`. It holds:
- a `running` flag (prevents overlapping populate/reset/sync jobs)
- row counts for every table (including `no_shows` and `cancellations`)
- `populate_progress`, `reset_progress`, `dynamo_progress`, `timelapse` sub-states

When acquiring a lock, **never `.unwrap()`** — see §7.

### Databases
- **Aurora DSQL** is authoritative. 16 tables in the `vital_fold` schema + `public.users`.
- **DynamoDB** holds read-optimized copies of `patient_visit` + `patient_vitals` only, written from Aurora. Documented in [docs/dynamo.md](docs/dynamo.md).

### Authentication
JWT bearer tokens validated in [vital-fold-engine/src/middleware/auth.rs](vital-fold-engine/src/middleware/auth.rs). Public routes: `/`, `/health`, `/api/v1/auth/login`, `/api/v1/auth/admin-login`, Swagger UI, static assets. Everything else requires a valid token.

---

## 7. Invariants — hard rules, do not violate

### 7.1 No bare `.unwrap()` / `.expect()`
From [CONTRIBUTING.md:45](CONTRIBUTING.md#L45):

> Never use bare `.unwrap()` or `.expect()` — prefer `?`, `.ok_or_else()`, or `.unwrap_or_else()` with a safe fallback. This is enforced in review.

Use one of these instead:
- `?` operator when the function returns `Result`
- `.ok_or_else(|| AppError::...)?`
- `.unwrap_or_else(|e| { tracing::error!(...); default })`
- `.unwrap_or_default()` when a default is truly safe

Mutex/RwLock poisoning is handled by logging and recovering, never panicking.

### 7.2 Never weaken the synthetic-data disclaimer
The "Fictitious Data" notice in [README.md](README.md) is load-bearing — legal + reputational. Edit wording only when explicitly asked; never delete.

### 7.3 Appointment status gates downstream writes
Only `status = 'completed'` appointments produce visits, vitals, medical records, surveys, or CPT line-items. Heatmap and visitor endpoints filter to completed-only. Do not blindly change this without understanding the data-quality implications.

### 7.4 Nullable columns — keep them nullable
- `patient_vitals.height`, `weight`, `oxygen_saturation` — intentionally `NULL` in ~3% of rows.
- `patient_demographics.ssn` — intentionally has ~2% duplicates.
- `patient.email` — ~3% duplicates.
- `patient_insurance.policy_number` — ~1% duplicates.
- `patient.middle_name` — populated for ~40% of patients.
- Vital outliers (~2%), late arrivals (~2%), stale cached `age`, lapsed coverage — all intentional.

These are **features**, not bugs. They exist to stress-test downstream pipelines. Do not "fix" them.

### 7.5 RNG + async
`rand::ThreadRng` is `!Send`. In any async generator, drop the RNG before `.await`-ing. The existing generators follow this pattern — copy it.

### 7.6 Bulk inserts
Aurora DSQL has a per-statement size limit. Use `UNNEST` with a **2,500-row batch cap**. The existing generators already do this.

### 7.7 Do not introduce a frontend build step
The Preact UI loads via CDN ESM imports. No Webpack, no Vite, no npm. If a task seems to require one, stop and confirm with the user.

---

## 8. The 16 Aurora DSQL tables (vital_fold schema)

Schema source of truth: [vital-fold-engine/migrations/init.sql](vital-fold-engine/migrations/init.sql).

| # | Table | Purpose |
|---|---|---|
| 1 | `insurance_company` | 7 fictional carriers |
| 2 | `insurance_plan` | ~21 plans, FK to company |
| 3 | `provider` | ~50 providers; BIGINT identity PK |
| 4 | `clinic` | 10 fixed clinics; BIGINT identity PK |
| 5 | `patient` | 50,000 default; UUID PK |
| 6 | `emergency_contact` | 1:1 with patient |
| 7 | `patient_demographics` | 1:1 with patient; has the dup-SSN trap |
| 8 | `patient_insurance` | 1+ per patient; lapse window |
| 9 | `clinic_schedule` | provider × clinic × day |
| 10 | `appointment` | slot-grid; has `status` column |
| 11 | `medical_record` | completed-only |
| 12 | `patient_visit` | completed-only; carries `record_expiration_epoch` for Dynamo TTL |
| 13 | `patient_vitals` | completed-only; nullable height/weight/SpO₂ |
| 14 | `survey` | ~30% of visits |
| 15 | `cpt_code` | 12 E/M + EKG codes with work/pe/mp RVU |
| 16 | `appointment_cpt` | 1–2 line items per completed appointment |

Plus `public.users` (email, bcrypt password_hash) for dashboard login.

---

## 9. Documentation map — where to look

| Doc | When to open it |
|---|---|
| [README.md](README.md) | Project pitch, disclaimer, endpoint summary |
| [vital-fold-engine/API.md](vital-fold-engine/API.md) | All 22 endpoints with curl examples |
| [vital-fold-engine/ARCHITECTURE.md](vital-fold-engine/ARCHITECTURE.md) | Request flow, state, phases, scaling |
| [vital-fold-engine/DEVELOPMENT.md](vital-fold-engine/DEVELOPMENT.md) | Local setup, code style, adding features |
| [vital-fold-engine/INSTALLATION.md](vital-fold-engine/INSTALLATION.md) | Aurora DSQL + DynamoDB + IAM + Docker + Render setup |
| [vital-fold-engine/QUICKSTART.md](vital-fold-engine/QUICKSTART.md) | 5-minute bootstrap (DSQL path) |
| [docs/models-spec.md](docs/models-spec.md) | Rust struct ↔ table mapping |
| [docs/dynamo.md](docs/dynamo.md) | DynamoDB schema & TTL |
| [docs/frontend.md](docs/frontend.md) | Preact dashboard components |
| [docs/airflow-integration.md](docs/airflow-integration.md) | Orchestration examples |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Expectations for PRs — read before opening one |
| [CHANGELOG.md](CHANGELOG.md) | Record every user-visible change under `[Unreleased]` |

**When changing code that is described in docs, update the relevant doc in the same commit.** Stale docs have already bitten this repo once.

---

## 10. Working-with-this-repo checklist

Before you call a task "done":

1. `cargo check` — zero errors, zero warnings
2. `cargo test --all-targets` — everything green (currently 24 tests)
3. `cargo clippy --all-targets --all-features -- -D warnings` — clean (CI gates on this)
4. `cargo fmt --check` — clean
5. No new `.unwrap()` or `.expect()` introduced (§7.1); no `unsafe` (forbidden crate-wide)
6. If you changed behavior visible to users, update [CHANGELOG.md](CHANGELOG.md) under `[Unreleased]`
7. If you changed schema, API, models, or architecture, update the corresponding doc in the same commit
8. If you changed row counts, status shapes, or JSON responses, update [vital-fold-engine/API.md](vital-fold-engine/API.md) examples
9. If you created, moved, renamed, or deleted any file or directory, update (or create) the touched directories' `map.md` in the same change (§3.1)

---

## 11. Things to confirm with the user before doing

- Destructive git operations (`reset --hard`, `push --force`, deleting branches)
- Merging to `main` (always ask, even after a clean build)
- Adding new dependencies (especially anything that changes the build model — e.g. a frontend bundler)
- Removing or rewording the fictitious-data disclaimer
- Changing the no-unwrap policy or its enforcement
- Changing the default row counts (they're tuned for realism and pipeline stress)

---

*Last updated: 2026-04-19. Keep this file current when project-level conventions change.*

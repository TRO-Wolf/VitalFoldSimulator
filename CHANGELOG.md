# VitalFold Engine — Change Log

> Single source of truth for all project changes.
> Update this document with every commit or significant modification.

---

## [Unreleased] — feature/web (uncommitted)

**Geographic Patient Distribution + Spec Sync + Documentation Overhaul**

### Code Changes

#### Geographic Patient Distribution
Patient addresses are now geographically correlated with clinic locations instead of
fully random US addresses. Each patient is assigned a "home" clinic using weighted random
proportional to approximate metro population (Miami/Atlanta ~14, Asheville/Tallahassee ~2-3).
Appointment clinic assignment biased 70% toward home clinic, 30% random.

- **patient.rs:** Added `METRO_AREAS` constant with city/state/zip/weight for all 10 clinics;
  `build_patient_batch()` uses `WeightedIndex` for proportional assignment
- **appointment.rs:** `generate_appointments()` now checks `patient_home_clinics` for 70/30 bias
- **mod.rs:** Added `patient_home_clinics: Vec<usize>` to `SimulationContext`

#### DynamoDB Count Fix (prior session)
Replaced `describe_table().item_count()` (approximate, ~6h stale) with `Scan` using
`Select::Count` for exact DynamoDB counts in `GET /simulate/db-counts`.

### Spec Synchronization (claude.md)
- Updated all 21 endpoints in API section
- Added `patient_visit` and `patient_vitals` tables to schema list
- Updated `SimulationCounts` to show all 15 fields
- Fixed Florida clinic count (5→6)
- Removed DuckDB/Polars from tech stack
- Removed `SimulateResponse` struct (uses `MessageResponse`)
- Updated DELETE→POST for `/simulate/reset`
- Documented admin-login, removed registration references
- Documented both DynamoDB tables with correct attribute names

### Documentation Overhaul
- **README.md:** Full rewrite — 21-endpoint table, three-phase lifecycle diagram, frontend
  dashboard section, documentation index, configuration table
- **API.md:** Full rewrite — all 21 endpoints with curl examples, request/response JSON, error
  reference, polling pattern guide
- **dynamo.md:** Expanded from 30 to 150+ lines — dual table schemas, key design, write strategy,
  TTL, capacity estimates, reset mechanics
- **ARCHITECTURE.md:** Added four sections — Three-Phase Data Lifecycle, Frontend Dashboard,
  Progress Tracking, Visualization Pipeline
- Deleted duplicate `vital-fold-engine/README.md` and `vital-fold-engine/DOCUMENTATION.md`

---

## Prior Unreleased — feature/web

**Admin-Only API + Aurora-First Visits/Vitals + Date-Range Simulation + Production Hardening**

### Architectural Changes

#### 0. Patient Vitals Pivot — EAV to Wide Columns
Eliminated the `patient_vitals` table (EAV pattern: 7 rows per visit) by embedding vital
measurements as columns directly on `patient_visits`. Reduces Aurora row count by 7x for
vitals and removes the DynamoDB `patient_vitals` table entirely.

- **Schema:** Added 7 columns to `patient_visits` (`height`, `weight`, `blood_pressure`,
  `heart_rate`, `temperature`, `oxygen_saturation`, `pulse_rate`); deleted `patient_vitals`
  table and all 4 indexes
- **Model:** Expanded `PatientVisit` struct with 7 vital fields; deleted `PatientVital` struct
  and `src/models/patient_vital.rs`
- **Generator:** `visit.rs` now generates vitals inline during visit INSERT (17-column UNNEST);
  removed `generate_patient_vitals()` and `generate_vitals_for_visits()`
- **Pipeline:** Legacy populate steps 13→12, dynamic steps 5→4; removed `visit_ids`/`visit_data`
  from `SimulationContext`
- **DynamoDB:** `write_patient_visit()` embeds 7 vital attributes on the visit item;
  deleted `write_patient_vitals()` function and `patient_vitals` table from reset
- **Counts:** Removed `patient_vitals` and `dynamo_patient_vitals` from `SimulationCounts`
- **Frontend:** Removed vitals entries from Aurora and DynamoDB count tables on dashboard

#### 1. Admin-Only API — Removed User Registration
- Deleted `RegisterRequest` struct and `validate()` method from `models/user.rs`
- Deleted `register` handler (86 lines) from `handlers/auth.rs`
- Removed `POST /api/v1/auth/register` route from `routes.rs`
- Removed `auth::register` from OpenAPI paths and `RegisterRequest` from schemas in `main.rs`
- Cleaned `bcrypt` imports: `{hash, verify, DEFAULT_COST}` → `verify` only
- Updated OpenAPI tag: "User registration and login" → "User login"

#### 2. Aurora-First Visits (with Embedded Vitals)
Phase 1 (`POST /populate`) now generates `patient_visits` (with inline vitals) in Aurora DSQL.
Phase 2 (`POST /simulate`) reads from Aurora and writes to DynamoDB — no more random on-the-fly generation.

- **New model:** `src/models/patient_visit.rs` — `PatientVisit` struct (`sqlx::FromRow`, `BigDecimal` copay, 7 vital fields)
- **New generator:** `src/generators/visit.rs`:
  - `generate_patient_visits()` — one visit per appointment with inline vitals, randomized checkout (30-120 min), provider seen (5-30 min), EKG (20%), copay ($20-$150)
  - `generate_visits_for_appointments()` — standalone variant for date-range flow
  - All use 2500-row UNNEST batches with `INSERT ... RETURNING` (17 columns)
- `run_populate` expanded from 11 to 12 steps (added patient_visits generation)
- `run_simulate` rewritten: queries visits from Aurora for today, writes to DynamoDB with bounded concurrency
- `write_patient_visit()` rewritten: accepts Aurora `PatientVisit` struct with embedded vitals, no random generation
- `SimulationCounts` expanded: `patient_visits: usize` (Aurora count), `dynamo_patient_visits: usize`
- Aurora schema updated: `patient_visits` table (with vital columns) + indexes added to `health_clinic_schema.sql`
- Reset handler updated: deletes `patient_visits` before appointments

### Added

#### Live Database Counts
- `GET /simulate/db-counts` endpoint — queries live record counts from both Aurora DSQL and DynamoDB
- Aurora: single SQL query with 13 scalar `COUNT(*)` subqueries (one per table)
- DynamoDB: `describe_table` on both tables (approximate counts, ~6hr AWS refresh)
- `describe_table_count()` helper with graceful fallback to 0 on error
- Dashboard "Refresh from DB" button — fetches live counts and overrides in-memory status counts
- Live counts auto-clear when any populate/simulate action starts, so in-memory progress is visible during operations

#### Date-Range DynamoDB Sync
- `POST /simulate/date-range` endpoint — sync existing Aurora visit data to DynamoDB for a caller-specified date range (requires prior Dynamic Populate run)
- `DateRangeRequest` struct with validation (date format, start <= end, max 90-day span)
- Pre-flight validation: checks that visits exist in Aurora before starting sync
- `run_date_range_simulate()` in `generators/mod.rs` — reads patient_visit JOIN patient_vitals from Aurora, writes to both DynamoDB tables
- `DateRangeForm` Preact component (`static/js/components/date-range-form.js`) — date pickers only, no generation parameters
- Dashboard integration: `dateRangeConfig` state, `handleDateRangeSimulate()` action

#### Configurable Populate Dates
- `start_date` and `end_date` fields added to `SimulationConfig` (default: tomorrow to tomorrow+89 days)
- `PopulateRequest` extended to accept `start_date`/`end_date` strings
- Date validation occurs before `try_start()` to prevent stuck state on parse errors
- Appointments distributed across the configurable range instead of fixed 0-89 day offset

#### Populate Progress Tracking
- `PopulateProgress` struct in `engine_state.rs`: `current_step`, `steps_done`, `total_steps`, `rows_written`, `is_complete`
- `POPULATE_TOTAL_STEPS` (12), `POPULATE_STEP_NAMES` array, `set_populate_step()` helper in `generators/mod.rs`
- `count_aurora_rows()` helper sums all Aurora count fields for progress tracking
- `populate_progress: Mutex<Option<PopulateProgress>>` field on `SimulatorState`
- `populate_progress` optional field on `SimulationStatusResponse` with `#[serde(skip_serializing_if = "Option::is_none")]`
- Dashboard UI: progress bar with step name, step counter, cumulative rows written

#### Reset Progress Tracking
- `ResetProgress` struct in `engine_state.rs`: `current_table`, `tables_done`, `total_tables`, `rows_deleted`, `is_complete`
- `reset_progress: Mutex<Option<ResetProgress>>` field on `SimulatorState`
- `reset_progress` optional field on `SimulationStatusResponse`
- `reset_data` handler rewritten with exponential backoff retry for Aurora DSQL OC000 optimistic concurrency errors
- Dashboard UI: progress bar with current table name, table counter, cumulative rows deleted

### Fixed (Production Hardening)

- **DynamoDB throttling resilience** — resolved `ThrottlingException` (4,000 WCU on-demand limit) during high-volume writes
  - Lowered `DYNAMO_CONCURRENCY` from 128 → 40 in both `run_simulate` and `run_date_range_simulate` (stays within 4,000 WCU/table)
  - Added retry with exponential backoff + equal jitter to `write_patient_visit()` and `write_patient_vitals()` (up to 5 retries: 25-50ms, 50-100ms, 100-200ms, 200-400ms, 400-800ms)
  - Throttle detection via AWS error codes (`ThrottlingException`, `ProvisionedThroughputExceededException`)
  - Retrying tasks hold semaphore permits during backoff, providing natural backpressure
  - `debug`-level logging on each retry for troubleshooting without log flooding
- **`AppError::Conflict` variant** — new error variant in `errors.rs` mapping to HTTP 409 (`StatusCode::CONFLICT`)
  - All 7 "already in progress" / "cannot reset while running" errors changed from `AppError::BadRequest` (400) to `AppError::Conflict` (409)
  - 2 utoipa annotations fixed from `status = 400` to `status = 409` (timelapse, replay)
- **Eliminated all naked `.unwrap()` calls** across 6 files (17 total):
  - 4 semaphore acquires (`generators/mod.rs`) → `.map_err(|_| AppError::Internal(...))?`
  - 4 chrono constructors (`generators/appointment.rs`, `generators/clinic.rs`) → `.expect("reason")`
  - 1 date constructor (`generators/insurance.rs`) → `.expect("2024-01-01 is a valid date")`
  - 6 mutex locks (`engine_state.rs`) → `.expect("state mutex poisoned")`
  - 2 tracing directive parses (`main.rs`) → `.expect("valid tracing directive")`
- **rand API migration** — updated deprecated calls across generators:
  - `gen_range()` → `random_range()`, `gen_bool()` → `random_bool()`, `thread_rng()` → `rng()`
  - Affected: `generators/insurance.rs`, `generators/patient.rs`

### Removed (Documentation Consolidation)
- Deleted 8 standalone docs, consolidated into 4 new files:
  - `docs/CODE_REVIEW_SUMMARY.md`, `docs/CODE_VALIDATION_REPORT.md`, `docs/FIXES_TRACKING.md`, `docs/MISTAKE_ANALYSIS.md` → `docs/BUILD_HISTORY.md`
  - `docs/DEPRECATION_UPDATE_PLAN.md`, `docs/DEPRECATION_UPDATE_COMPLETION.md` → `docs/DEPRECATION_HISTORY.md`
  - `docs/project.md`, `docs/synthetic_data.md` → `docs/project-origins.md`
  - Frontend docs (`docs/front/architecture.md`, `components.md`, `implementation.md`, `pages.md`) → `docs/frontend.md`

### Files Changed
| File | Type |
|------|------|
| `src/models/patient_visit.rs` | **New** — `PatientVisit` struct (with embedded vital fields) |
| `src/models/patient_vital.rs` | **Deleted** — EAV pattern replaced by wide columns on `PatientVisit` |
| `src/models/mod.rs` | Modified — added patient_visit module, removed patient_vital |
| `src/models/user.rs` | Modified — removed `RegisterRequest`, added `reset_progress`/`populate_progress` to status response |
| `src/generators/visit.rs` | **New** — visit Aurora generator (vitals embedded inline) |
| `src/generators/mod.rs` | Modified — +733 lines: visit module, context fields, 13-step populate, Aurora-first simulate, date-range, timelapse, progress tracking |
| `src/generators/appointment.rs` | Modified — configurable dates, rewritten DynamoDB write functions with retry + jitter for throttling |
| `src/generators/medical_record.rs` | Modified — added `generate_medical_records_for_range()` |
| `src/generators/clinic.rs` | Modified — `.unwrap()` → `.expect()` |
| `src/generators/insurance.rs` | Modified — `.unwrap()` → `.expect()`, rand API migration |
| `src/generators/patient.rs` | Modified — rand API migration, type annotations |
| `src/handlers/auth.rs` | Modified — removed `register` handler, cleaned imports |
| `src/handlers/simulation.rs` | Modified — +667 lines: date-range, timelapse, heatmap, visitors, replay, reset progress, populate progress, OC000 retry, Conflict errors |
| `src/engine_state.rs` | Modified — +113 lines: `TimelapseState`, `ClinicActivity`, `ResetProgress`, `PopulateProgress`, patient_visits counts, timelapse/reset/populate state |
| `src/errors.rs` | Modified — added `Conflict(String)` variant |
| `src/main.rs` | Modified — removed register, added timelapse/heatmap/visitors/replay/date-range to OpenAPI, static file serving |
| `src/routes.rs` | Modified — removed register route, added 6 new routes |
| `static/js/components/date-range-form.js` | **New** — date range form component |
| `static/js/components/populate-form.js` | Modified — added start/end date inputs |
| `static/js/pages/dashboard.js` | Modified — added populate/reset progress, date-range config, count table fields |
| `static/css/style.css` | Modified — added reset-progress and populate-progress styles |
| `docs/health_clinic_schema.sql` | Modified — added `patient_visits` table (with vital columns) + indexes |
| `docs/BUILD_HISTORY.md` | **New** — consolidated build history |
| `docs/DEPRECATION_HISTORY.md` | **New** — consolidated deprecation history |
| `docs/project-origins.md` | **New** — consolidated project origins |
| `docs/frontend.md` | **New** — consolidated frontend architecture docs |

---

## [32cc4bb] — 2026-03-21 — `latest`

**Frontend SPA + Admin Dashboard + Timelapse/Replay/Visitors/Heatmap**

### Added
- Complete Preact + HTM frontend (no build step, CDN-based ESM imports)
  - `static/index.html` — SPA entry point with Pico CSS
  - `static/css/style.css` — custom styling (heatmap grid, dashboard layout, progress bars)
  - `static/js/app.js` — hash-based router (`#login`, `#dashboard`)
  - `static/js/api.js` — fetch wrapper with JWT bearer token injection
- **Pages:**
  - `static/js/pages/login.js` — login/register forms with token storage
  - `static/js/pages/dashboard.js` — main admin UI with populate, simulate, stop, reset controls
  - `static/js/pages/visitors.js` — per-clinic visitor list with patient names
- **Components:**
  - `static/js/components/confirm-modal.js` — confirmation dialog for destructive actions
  - `static/js/components/count-table.js` — Aurora/DynamoDB count display
  - `static/js/components/heatmap.js` — 238-line real-time clinic activity heatmap with color scale
  - `static/js/components/nav.js` — navigation bar with logout
  - `static/js/components/populate-form.js` — configurable populate parameters
  - `static/js/components/status-badge.js` — running/idle status indicator
- `Sonnet.md` — workflow orchestration guidelines for Claude Sonnet
- Frontend architecture documentation (`docs/front/`):
  - `architecture.md` — stack rationale, auth flow, routing
  - `components.md` — component API documentation
  - `implementation.md` — integration details
  - `pages.md` — page-level documentation

### Added (Backend)
- `POST /simulate/timelapse` — single-day heatmap with auto-populated DynamoDB data
- `GET /simulate/heatmap` — poll per-clinic activity for real-time heatmap
- `GET /simulate/visitors` — today's visitors grouped by clinic (patient names)
- `POST /simulate/replay` — read-only heatmap replay (no DynamoDB writes)
- `POST /simulate/replay-reset` — clear replay state
- `TimelapseState`, `ClinicActivity` structs in `engine_state.rs`
- `run_timelapse()`, `run_today_heatmap()`, `run_heatmap_replay()` in `generators/mod.rs`
- Heatmap, visitors, replay handlers in `handlers/simulation.rs`
- `actix-files` dependency for serving static frontend

### Changed
- `main.rs` — added Swagger UI, OpenAPI schema expansion, static file serving
- `routes.rs` — added timelapse, heatmap, visitors, replay routes
- `generators/insurance.rs` — minor refactoring
- `generators/patient.rs` — minor refactoring
- `engine_state.rs` — expanded with timelapse and heatmap state

---

## [ae104e0] — 2026-03-07 — `claude optimization plan`

**Deprecation Cleanup + Dependency Pruning + Source Documentation**

### Removed
- `duckdb` dependency (unused, added 15s+ to build time)
- `polars` dependency (unused, massive transitive deps)
- `tokio-postgres` dependency (redundant — sqlx handles connections)
- Dead `errors.rs` variants that were never constructed

### Fixed
- **JWT algorithm validation** — `Validation::default()` accepted any algorithm; changed to `Validation::new(Algorithm::HS256)` to prevent algorithm confusion attacks
- **Deprecated `chrono::Duration`** — replaced all occurrences with `chrono::TimeDelta` across generators (appointment, clinic, medical_record, patient)

### Added
- Source-level documentation files (`src_*_claude.md`) in each module directory:
  - `src/db/src_db_claude.md` (184 lines)
  - `src/generators/src_generators_claude.md` (525 lines)
  - `src/handlers/src_handlers_claude.md` (460 lines)
  - `src/middleware/src_middleware_claude.md` (166 lines)
  - `src/models/src_models_claude.md` (501 lines)
- `docs/DEPRECATION_UPDATE_PLAN.md` — detailed plan for all fixes
- `docs/DEPRECATION_UPDATE_COMPLETION.md` — completion report
- `docs/skills/Haiku.md` — Claude Haiku workflow guidelines

### Changed
- `claude.md` — synced specification with actual implementation
- `Cargo.toml` — removed 3 unused dependencies
- `Cargo.lock` — ~2,800 lines removed (transitive deps pruned)
- `middleware/auth.rs` — hardened JWT validation
- `models/appointment.rs` — added 7 lines (field additions)
- `models/clinic.rs` — minor type fix

---

## [4cd6c90] — 2026-02-25 — `added need admin endpoint and dynamo endpoints`

**Admin Authentication + DynamoDB Integration**

### Added
- `POST /auth/admin-login` — admin-only login endpoint with separate secret validation
- `AdminLoginRequest` struct in `handlers/auth.rs`
- Admin role claim in JWT middleware (`is_admin` field in Claims)
- DynamoDB configuration in `config.rs` (table names, region)
- `docs/dynamo.md` — DynamoDB table schema (patient_visit with embedded vitals)
- `write_patient_visit()` in `generators/appointment.rs`

### Changed
- `handlers/simulation.rs` — added DynamoDB write trigger
- `generators/appointment.rs` — expanded with DynamoDB write functions (+41/-13)
- `generators/mod.rs` — integrated DynamoDB writes into simulation pipeline (+28/-4)
- `main.rs` — DynamoDB client initialization, OpenAPI updates
- `middleware/auth.rs` — admin role checking (+12)
- `models/clinic.rs` — field type adjustment
- `routes.rs` — admin login route, DynamoDB endpoints

---

## [570c359] — 2026-02-24 — `added multiple endpoints`

**Major Endpoint Expansion + Generator Rewrite**

### Added
- `POST /simulate` — write DynamoDB records for today's appointments (Phase 2)
- `POST /simulate/stop` — stop running simulation
- `GET /simulate/status` — poll run status and record counts
- `POST /simulate/reset` — delete all Aurora DSQL data
- Full simulation lifecycle with `SimulatorState` (AtomicBool running flag)

### Changed (Major Rewrites)
- `generators/appointment.rs` — rewritten with UNNEST bulk inserts, DSQL_BATCH_SIZE chunking (+269/-93 net)
- `generators/clinic.rs` — expanded with realistic clinic data (+156/-119 net)
- `generators/insurance.rs` — expanded with realistic insurance companies (+138/-127 net)
- `generators/medical_record.rs` — rewritten with diagnosis/treatment logic (+146/-109 net)
- `generators/mod.rs` — full orchestration with `run_populate` and `run_simulate` (+266/-199 net)
- `generators/patient.rs` — expanded with realistic demographic data (+435/-190 net)
- `generators/provider.rs` — expanded with specialties (+86/-82 net)
- `handlers/simulation.rs` — expanded from 122 to 486+ lines with all control endpoints
- `db/mod.rs` — pool configuration changes
- `engine_state.rs` — state tracking additions
- `main.rs` — OpenAPI expansion
- `routes.rs` — complete route restructuring
- `claude.md` — specification updates (+49)

---

## [5838add] — 2026-02-22 — `added first major build` (file moves)

**Documentation Reorganization**

### Changed
- Moved root-level docs into `docs/` directory:
  - `CODE_REVIEW_SUMMARY.md` -> `docs/CODE_REVIEW_SUMMARY.md`
  - `CODE_VALIDATION_REPORT.md` -> `docs/CODE_VALIDATION_REPORT.md`
  - `FIXES_TRACKING.md` -> `docs/FIXES_TRACKING.md`
  - `MISTAKE_ANALYSIS.md` -> `docs/MISTAKE_ANALYSIS.md`
  - `health_clinic_schema.sql` -> `docs/health_clinic_schema.sql`
  - `project.md` -> `docs/project.md`
  - `synthetic_data.md` -> `docs/synthetic_data.md`

---

## [087ee32] — 2026-02-22 — `added first major build` (implementation)

**Production Application Build — Full Rust Backend + Documentation Suite**

### Added
- **Application documentation** (6 files, ~3,500 lines total):
  - `vital-fold-engine/API.md` — complete API reference (725 lines)
  - `vital-fold-engine/ARCHITECTURE.md` — technical architecture (1,105 lines)
  - `vital-fold-engine/DEVELOPMENT.md` — developer guide (823 lines)
  - `vital-fold-engine/DOCUMENTATION.md` — feature documentation (483 lines)
  - `vital-fold-engine/INSTALLATION.md` — installation guide (664 lines)
  - `vital-fold-engine/QUICKSTART.md` — quick start (213 lines)
  - `vital-fold-engine/README.md` — project README (486 lines)
- `.env.example` — 73 configuration variables documented
- Database migrations:
  - `migrations/001_init.sql` — users table
  - `migrations/health_clinic_schema.sql` — full vital_fold schema (474 lines)

### Changed (Refinements to Initial Build)
- `README.md` (root) — expanded from 1 line to 487 lines
- All source files refined: db, engine_state, generators, handlers, middleware, models, routes, main
- `Cargo.toml` — dependency version updates
- `Cargo.lock` — updated

---

## [d966f30] — 2026-02-22 — `Add comprehensive code review and validation documentation`

**Code Review Documentation**

### Added
- `CODE_REVIEW_SUMMARY.md` — 285-line review covering:
  - 5 critical production fixes applied
  - 6 data generators validated (1,100+ lines)
  - 4 simulation endpoints reviewed
  - Security, type safety, async/concurrency checks
  - Domain value validation (100% match)
  - Production readiness checklist

---

## [31e17aa] — 2026-02-22 — `Complete Steps 10-18: Generators, simulation handler, and routes`

**Initial Full Implementation — 40 Files, 12,804 Lines**

### Added
- `.gitignore` — Rust/IDE ignores
- `claude.md` — authoritative project specification (1,240 lines)
- `claude.md.bak` — backup of specification
- **Quality assurance docs:**
  - `CODE_VALIDATION_REPORT.md` — 9-file validation report (283 lines)
  - `FIXES_TRACKING.md` — 5 critical fixes tracked (174 lines)
  - `MISTAKE_ANALYSIS.md` — Actix extractor mistake analysis (98 lines)
- **Domain specifications:**
  - `docs/models-spec.md` — data model definitions (394 lines)
  - `health_clinic_schema.sql` — PostgreSQL schema (487 lines)
  - `project.md` — original project prompt (22 lines)
  - `synthetic_data.md` — generation parameters (39 lines)
- **Rust application** (`vital-fold-engine/`):
  - `Cargo.toml` + `Cargo.lock` — 17+ dependencies
  - `src/config.rs` — environment configuration (102 lines)
  - `src/db/mod.rs` — Aurora DSQL connection pool with IAM auth (102 lines)
  - `src/engine_state.rs` — simulation state management (146 lines)
  - `src/errors.rs` — AppError enum with Actix ResponseError impl (116 lines)
  - `src/main.rs` — Actix server entry point (78 lines)
  - `src/routes.rs` — route registration (61 lines)
  - **Models** (7 files): user, patient, provider, clinic, appointment, insurance, medical_record
  - **Handlers** (4 files + mod): health, auth, user, simulation
  - **Middleware** (1 file + mod): JWT validation with Claims extraction
  - **Generators** (6 files + mod): insurance, clinic, provider, patient, appointment, medical_record

---

## [3305d7d] — 2026-02-20 — `first commit`

**Repository Creation**

### Added
- `README.md` — initial 1-line placeholder

---

## Document Index — All Markdown Files

### Root
| File | Purpose | Created |
|------|---------|---------|
| `README.md` | Project overview | 2026-02-20 |
| `claude.md` | Authoritative specification | 2026-02-22 |
| `Sonnet.md` | Claude Sonnet workflow guidelines | 2026-03-21 |
| `CHANGELOG.md` | This document — change tracking | 2026-03-22 |

### docs/
| File | Purpose | Created |
|------|---------|---------|
| `BUILD_HISTORY.md` | Initial build: fixes, validation, lessons | 2026-02-22 (consolidated 03-22) |
| `DEPRECATION_HISTORY.md` | Dependency pruning & deprecation fixes | 2026-03-07 (consolidated 03-22) |
| `project-origins.md` | Original prompt & synthetic data definitions | 2026-02-22 (consolidated 03-22) |
| `frontend.md` | Frontend architecture, components, pages, guide | 2026-03-21 (consolidated 03-22) |
| `dynamo.md` | DynamoDB table schemas (patient_visit) | 2026-02-25 |
| `models-spec.md` | Rust model definitions for all DB tables | 2026-02-22 |
| `health_clinic_schema.sql` | Aurora DSQL schema | 2026-02-22 |
| `dynamo.json` | DynamoDB table JSON spec | 2026-02-22 |
| `skills/Haiku.md` | Claude Haiku workflow guidelines | 2026-03-07 |

### vital-fold-engine/
| File | Purpose | Created |
|------|---------|---------|
| `README.md` | Engine-specific README | 2026-02-22 |
| `API.md` | Complete API reference | 2026-02-22 |
| `ARCHITECTURE.md` | Technical architecture | 2026-02-22 |
| `DEVELOPMENT.md` | Developer guide | 2026-02-22 |
| `DOCUMENTATION.md` | Feature documentation | 2026-02-22 |
| `INSTALLATION.md` | Installation guide | 2026-02-22 |
| `QUICKSTART.md` | Quick start guide | 2026-02-22 |

### Source-level references (vital-fold-engine/src/)
| File | Purpose | Created |
|------|---------|---------|
| `db/src_db_claude.md` | DB module reference | 2026-03-07 |
| `generators/src_generators_claude.md` | Generators reference | 2026-03-07 |
| `handlers/src_handlers_claude.md` | Handlers reference | 2026-03-07 |
| `middleware/src_middleware_claude.md` | Middleware reference | 2026-03-07 |
| `models/src_models_claude.md` | Models reference | 2026-03-07 |

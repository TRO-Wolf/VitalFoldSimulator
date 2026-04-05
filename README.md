# VitalFold Engine

> Synthetic healthcare data generation and simulation engine for cardiac clinic data pipelines.

[![Rust](https://img.shields.io/badge/Rust-1.80+-orange.svg)](https://www.rust-lang.org/)
[![Actix-web](https://img.shields.io/badge/Actix--web-4.x-success.svg)](https://actix.rs/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Status](https://img.shields.io/badge/Status-Production--Ready-brightgreen.svg)]()

## What is VitalFold Engine?

VitalFold Engine is a Rust REST API that generates realistic synthetic patient data for **Vital Fold Health LLC**, a multi-region cardiac healthcare company headquartered in Florida. It populates Aurora DSQL (PostgreSQL-compatible) with relational clinical data, syncs visit records to DynamoDB for read-path optimization, and provides a real-time admin dashboard for visualization. The API is secured with JWT authentication and designed for orchestration via Apache Airflow.

---

## Three-Phase Data Lifecycle

```
Phase 1: Static Populate          Phase 2: Dynamic Populate         Phase 3: DynamoDB Sync
POST /populate/static              POST /populate/dynamic             POST /simulate/date-range
─────────────────────              ──────────────────────             ─────────────────────────
Aurora DSQL:                       Aurora DSQL:                       DynamoDB:
 • 7 insurance companies            • Clinic schedules                 • patient_visit table
 • 21 insurance plans                • Appointments                    • patient_vitals table
 • 10 clinics (SE US)                • Medical records
 • 50 providers                      • Patient visits                  Reads Aurora, writes DynamoDB.
 • 50,000 patients                   • Patient vitals                  No Aurora generation.
 • Emergency contacts                 • Surveys (~30% of visits)
 • Demographics                    Date-dependent data for a
 • Insurance links                 configurable date range.
                                   Requires Phase 1 first.
Reference data. Run once.
```

All phases are **fire-and-poll**: POST returns `202 Accepted`, work runs in a background task, poll `GET /simulate/status` until `running == false`.

---

## Quick Start

```bash
git clone https://github.com/TRO-Wolf/VitalFoldSimulator.git
cd vitalFoldEngine/vital-fold-engine
cp .env.example .env          # Edit with your DSQL endpoint + JWT secret
cargo build --release
cargo run --release            # Starts on http://0.0.0.0:8787
```

```bash
# Health check
curl http://localhost:8787/health

# Get a token
curl -X POST http://localhost:8787/api/v1/auth/admin-login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"your-admin-password"}'

# Seed reference data
curl -X POST http://localhost:8787/populate/static \
  -H "Authorization: Bearer <token>"
```

See [QUICKSTART.md](vital-fold-engine/QUICKSTART.md) for detailed setup or [INSTALLATION.md](vital-fold-engine/INSTALLATION.md) for deployment.

---

## API Endpoints (22 total)

Interactive docs available at `/swagger-ui/` when the server is running.

### Public

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Health check |
| POST | `/api/v1/auth/login` | User login (email + password) |
| POST | `/api/v1/auth/admin-login` | Admin login (env-var credentials) |

### User (JWT required)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/me` | Current user profile |

### Population — Phase 1 & 2 (JWT required)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/populate` | Legacy: run all 13 populate steps at once |
| POST | `/populate/static` | Phase 1: seed reference data (insurance, clinics, providers, patients) |
| POST | `/populate/dynamic` | Phase 2: seed date-dependent data (appointments, records, visits) |
| GET | `/populate/dates` | List dates that have been populated |
| POST | `/populate/reset-dynamic` | Delete dynamic data only, preserve reference data |

### Simulation — Phase 3 & Control (JWT required)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/simulate` | Sync today's Aurora visits to DynamoDB |
| POST | `/simulate/date-range` | Sync Aurora visits to DynamoDB for a date range |
| POST | `/simulate/stop` | Stop any running background task |
| GET | `/simulate/status` | Poll run status, counts, and progress |
| GET | `/simulate/db-counts` | Live record counts from Aurora + DynamoDB |
| POST | `/simulate/reset` | Delete all Aurora DSQL data |
| POST | `/simulate/reset-dynamo` | Delete all DynamoDB data |

### Visualization (JWT required)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/simulate/timelapse` | Start hour-by-hour heatmap animation |
| GET | `/simulate/heatmap` | Poll per-clinic activity during timelapse |
| POST | `/simulate/replay` | Read-only heatmap replay (no DynamoDB writes) |
| POST | `/simulate/replay-reset` | Clear replay state |
| GET | `/simulate/visitors` | Today's visitors grouped by clinic |

### Admin (JWT required)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/admin/init-db` | Drop and recreate the entire vital_fold schema from `migrations/init.sql` |

See [API.md](vital-fold-engine/API.md) for full request/response examples with curl commands.

---

## Frontend Dashboard

The engine serves a built-in admin SPA at the root URL (`/`). No build step required — Preact + HTM loaded via CDN.

**Features:**
- Login with admin credentials or user email/password
- Real-time database counts (Aurora + DynamoDB) with refresh
- Populate controls: configure patient count, provider count, date range, per-clinic weights
- Progress bars for populate, reset, and DynamoDB sync operations
- Clinic activity heatmap with hour-by-hour timelapse animation (8 AM – 5 PM)
- Per-clinic visitor list with patient names
- **Init Database** button — drops and recreates the entire schema from `migrations/init.sql`
- Dark theme

See [frontend.md](docs/frontend.md) for architecture details.

---

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Language | Rust (stable, 2021 edition) |
| Web Framework | Actix Web 4 |
| Database | Aurora DSQL (PostgreSQL-compatible, serverless) |
| Replication | DynamoDB (on-demand, two tables) |
| Auth | JWT (HS256) + bcrypt |
| Data Generation | `fake` crate v4 |
| OpenAPI | utoipa 5 + Swagger UI |
| Frontend | Preact 10 + HTM 3 + Pico CSS (CDN, no build step) |
| Deployment | Render.com |

---

## Configuration

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DSQL_CLUSTER_ENDPOINT` | Yes | — | Aurora DSQL hostname |
| `JWT_SECRET` | Yes | — | HMAC secret (min 32 chars) |
| `ADMIN_USERNAME` | No | — | Admin login username |
| `ADMIN_PASSWORD` | No | — | Admin login password |
| `HOST` | No | `0.0.0.0` | Bind address |
| `PORT` | No | `8787` | Bind port |
| `DSQL_REGION` | No | `us-east-1` | AWS region for IAM token signing |
| `DSQL_DB_NAME` | No | `postgres` | Database name |
| `DSQL_USER` | No | `admin` | Database user |
| `DB_POOL_SIZE` | No | `10` | Connection pool size |
| `JWT_EXPIRY_HOURS` | No | `24` | Token lifetime in hours |
| `RUST_LOG` | No | `info` | Log level filter |

See [INSTALLATION.md](vital-fold-engine/INSTALLATION.md) for full setup and deployment instructions.

---

## Synthetic Data

The simulator generates cardiac clinic data across 10 clinics in the southeastern US:

- **Clinics:** Charlotte, Asheville (NC) · Atlanta ×2 (GA) · Tallahassee, Miami ×2, Orlando, Jacksonville ×2 (FL)
- **Insurance:** 7 fictional carriers (Orange Spear, Care Medical, Cade Medical, Multiplied Health, Octi Care, Tatnay, Caymana)
- **Diagnoses:** 8 cardiac codes (AFib, CAD, Chest Pain, Hypertension, Hyperlipidemia, SOB, Tachycardia, Bradycardia)
- **Defaults:** 50,000 patients, 50 providers, 3 plans/company

### Provider-Driven Appointment Model
Appointment volume is **deterministic**, not configurable:
- Each provider fills **36 appointment slots per day** (8:00 AM to 4:45 PM in 15-minute windows)
- A clinic with 4 providers generates 144 appointments/day
- With 50 providers × 90 days = 162,000 appointments per populate run

### Per-Clinic Distribution (clinic_weights)
Patients, providers, and appointments are distributed across clinics by configurable weights:
- **Default:** `[12, 3, 14, 14, 2, 14, 14, 12, 8, 8]` (Charlotte, Asheville, Atlanta×2, Tallahassee, Miami×2, Orlando, Jacksonville×2)
- **Result:** Miami/Atlanta clinics get ~7 providers each (~252 appts/day); Asheville gets 1-2 (~36-72 appts/day)

### Provider Details
- **License types:** ~30% Nurse Practitioners (NP), ~70% MD/DO split evenly
- **Email format:** `j.smith@example.org` (first initial + last name)
- **Specialties:** Cardiologist, Cardiac Surgeon, Electrophysiologist, Interventional Cardiologist

### Clinic Details
- **Email format:** `vfhc_miami1@vitalfold.org` (VFHC prefix + city + number for duplicates)
- **Addresses:** Realistic format `1234 Elm Blvd, Suite 200`
- **Zip codes:** Metro-area prefix + 2 digits (e.g., Miami → `331xx`)

### Visit Timing
- `checkin_time`: 5-15 minutes **before** scheduled appointment (early arrival)
- `provider_seen_time`: 0-5 minutes after scheduled time
- `checkout_time`: 15-30 minutes after scheduled time

### Copay Structure
- **EKG visit (~20%):** $150-$350
- **Standard visit:** $20-$150

### Insurance Coverage
- `coverage_start_date` is a random date within the past 365 days

### Identity Columns (Aurora DSQL)
- `provider.provider_id` and `clinic.clinic_id` are **BIGINT** identity columns with `CACHE 1` (small tables, tight ordering)
- All other IDs (`patient_id`, `appointment_id`, etc.) remain **UUID**

### Surveys

Roughly **30% of patient visits** also generate a `vital_fold.survey` row containing `gene_prissy_score` (1–10), `experience_score` (1–10), and an optional `feedback_comments` free-text field. Surveys are the 6th step of `POST /populate/dynamic`. The intent is a gold-layer aggregation like `AVG(gene_prissy_score) GROUP BY provider_id` as a provider-quality metric.

### RVU / Productivity Metrics

Every appointment automatically generates **billing line-items** in `vital_fold.appointment_cpt` during dynamic populate — the industry-standard US healthcare productivity grain:
- `vital_fold.cpt_code` — reference table seeded by `POST /admin/init-db` with 12 common E/M + EKG CPT codes and their CY2024 work / PE / MP RVU values.
- `vital_fold.appointment_cpt` — line-item fact table: 1 E/M code per appointment (cardiology-weighted distribution, 99213/99214 dominant), plus a second row for CPT 93000 when `ekg_usage = true`. Each row snapshots the RVU components, Medicare conversion factor ($32.7442 for CY2024), and expected amount.

Gold-layer rollup:
```sql
SELECT provider_id,
       date_trunc('month', service_date) AS month,
       SUM(work_rvu_snapshot * units) AS total_wrvu,
       SUM(expected_amount)           AS expected_revenue
FROM vital_fold.appointment_cpt
GROUP BY provider_id, month;
```

---

## Documentation

| Document | Description |
|----------|-------------|
| [API Reference](vital-fold-engine/API.md) | All 22 endpoints with curl examples and response shapes |
| [Architecture](vital-fold-engine/ARCHITECTURE.md) | System design, data flow, module structure |
| [Development Guide](vital-fold-engine/DEVELOPMENT.md) | Local dev setup, code style, adding features |
| [Installation](vital-fold-engine/INSTALLATION.md) | Setup, deployment (Render, Docker), troubleshooting |
| [Quick Start](vital-fold-engine/QUICKSTART.md) | 5-minute setup guide |
| [Airflow Integration](docs/airflow-integration.md) | DAG examples for scheduling population + sync |
| [Frontend Architecture](docs/frontend.md) | SPA components, routing, state management |
| [DynamoDB Schema](docs/dynamo.md) | Table design, write strategy, TTL, capacity |
| [Data Models](docs/models-spec.md) | Rust struct definitions for all database tables |
| [Aurora Schema](vital-fold-engine/migrations/init.sql) | Full DDL for all 16 Aurora DSQL tables |
| [Changelog](CHANGELOG.md) | Release history and change log |

---

## License

MIT

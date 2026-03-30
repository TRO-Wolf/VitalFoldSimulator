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
 • Emergency contacts
 • Demographics                    Date-dependent data for a
 • Insurance links                 configurable date range.
                                   Requires Phase 1 first.
Reference data. Run once.
```

All phases are **fire-and-poll**: POST returns `202 Accepted`, work runs in a background task, poll `GET /simulate/status` until `running == false`.

---

## Quick Start

```bash
git clone https://github.com/your-org/vitalFoldEngine.git
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

## API Endpoints (21 total)

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

See [API.md](vital-fold-engine/API.md) for full request/response examples with curl commands.

---

## Frontend Dashboard

The engine serves a built-in admin SPA at the root URL (`/`). No build step required — Preact + HTM loaded via CDN.

**Features:**
- Login with admin credentials or user email/password
- Real-time database counts (Aurora + DynamoDB) with refresh
- Populate controls: configure patient count, provider count, date range
- Progress bars for populate, reset, and DynamoDB sync operations
- Clinic activity heatmap with hour-by-hour timelapse animation
- Per-clinic visitor list with patient names

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
- **Patients:** Geographically distributed — addresses and clinic assignments weighted by metro population
- **Defaults:** 50,000 patients, 50 providers, 3 plans/company, 2 appointments/patient

---

## Documentation

| Document | Description |
|----------|-------------|
| [API Reference](vital-fold-engine/API.md) | All 21 endpoints with curl examples and response shapes |
| [Architecture](vital-fold-engine/ARCHITECTURE.md) | System design, data flow, module structure |
| [Development Guide](vital-fold-engine/DEVELOPMENT.md) | Local dev setup, code style, adding features |
| [Installation](vital-fold-engine/INSTALLATION.md) | Setup, deployment (Render, Docker), troubleshooting |
| [Quick Start](vital-fold-engine/QUICKSTART.md) | 5-minute setup guide |
| [Airflow Integration](docs/airflow-integration.md) | DAG examples for scheduling population + sync |
| [Frontend Architecture](docs/frontend.md) | SPA components, routing, state management |
| [DynamoDB Schema](docs/dynamo.md) | Table design, write strategy, TTL, capacity |
| [Data Models](docs/models-spec.md) | Rust struct definitions for all database tables |
| [Aurora Schema](docs/health_clinic_schema.sql) | Full DDL for 13 Aurora DSQL tables |
| [Changelog](CHANGELOG.md) | Release history and change log |

---

## License

MIT

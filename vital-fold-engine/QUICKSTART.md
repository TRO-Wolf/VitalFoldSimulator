# Quick Start Guide

Get VitalFold Engine running in about 10 minutes against an Aurora DSQL cluster. For the full setup walkthrough (IAM policies, table creation, cost guidance), see [INSTALLATION.md](./INSTALLATION.md).

---

## 1. Prerequisites

- **Rust 1.80+** — `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Aurora DSQL cluster** with a reachable endpoint — see [INSTALLATION.md § AWS Prerequisites](./INSTALLATION.md#aws-prerequisites) to create one.
- **Two DynamoDB tables**: `patient_visit` and `patient_vitals` in the same region as your DSQL cluster.
- **AWS credentials** with `dsql:DbConnectAdmin` + DynamoDB write permissions. The default credential provider chain is used (env vars, `~/.aws/credentials`, EC2/ECS role).
- **JWT secret** — generate with `openssl rand -base64 32`.

VitalFold targets Aurora DSQL. Plain PostgreSQL is **not** supported at runtime.

---

## 2. Clone & Configure

```bash
git clone <your-repo-url>
cd vital-fold-engine
cp .env.example .env
```

Edit `.env` and set at minimum:

```env
DSQL_CLUSTER_ENDPOINT=<your-cluster-id>.dsql.us-east-1.on.aws
DSQL_REGION=us-east-1
JWT_SECRET=<openssl rand -base64 32>
ADMIN_USERNAME=admin
ADMIN_PASSWORD=<a strong password>
```

See `.env.example` for every supported variable and its default.

---

## 3. Run

```bash
cargo run --release
# Server listens on 0.0.0.0:8787
```

Health check:

```bash
curl http://127.0.0.1:8787/health
```

---

## 4. Initialize the Schema (first run only)

The engine ships with an admin endpoint that creates all 16 `vital_fold.*` tables (plus `public.users`) from `migrations/init.sql`. There is no `cargo sqlx migrate` step.

```bash
# 1) Get an admin token
TOKEN=$(curl -s -X POST http://127.0.0.1:8787/api/v1/auth/admin-login \
  -H 'Content-Type: application/json' \
  -d "{\"username\":\"$ADMIN_USERNAME\",\"password\":\"$ADMIN_PASSWORD\"}" \
  | jq -r .token)

# 2) Create the schema
curl -X POST http://127.0.0.1:8787/admin/init-db \
  -H "Authorization: Bearer $TOKEN"
```

Or click **Init Database** on the admin dashboard at `http://127.0.0.1:8787/`.

---

## 5. Populate & Simulate

The data lifecycle has three phases. Run them in order:

```bash
# Phase 1 — static reference data (insurance, providers, clinics, patients, CPT codes)
curl -X POST http://127.0.0.1:8787/populate/static \
  -H "Authorization: Bearer $TOKEN"

# Phase 2 — dynamic clinical activity for a date range
#   (appointments, visits, vitals, medical records, surveys, CPT line items)
curl -X POST http://127.0.0.1:8787/populate/dynamic \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"start_date":"2026-01-01","end_date":"2026-01-07"}'

# Phase 3 — sync completed visits + vitals to DynamoDB
curl -X POST http://127.0.0.1:8787/simulate/date-range \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"start_date":"2026-01-01","end_date":"2026-01-07"}'

# Poll status any time
curl http://127.0.0.1:8787/simulate/status \
  -H "Authorization: Bearer $TOKEN"
```

---

## 6. Interactive Exploration

- **Admin dashboard:** `http://127.0.0.1:8787/`
- **Swagger UI:** `http://127.0.0.1:8787/swagger-ui/` — click **Authorize** and paste your bearer token.

---

## Common Commands

```bash
cargo test --all-targets   # Run unit tests
cargo clippy --all-targets # Lint
cargo fmt                  # Format
cargo build --release      # Release binary at target/release/vital-fold-engine
```

---

## Troubleshooting

**`Failed to load configuration`** — `DSQL_CLUSTER_ENDPOINT` or `JWT_SECRET` is missing from `.env`. `JWT_SECRET` must be ≥32 chars.

**`Failed to create database pool`** — AWS credentials cannot be found, or the IAM principal lacks `dsql:DbConnectAdmin` on the target cluster ARN. Verify with `aws sts get-caller-identity` and check the policy in [INSTALLATION.md § AWS Prerequisites](./INSTALLATION.md#aws-prerequisites).

**`Port 8787 already in use`** — `lsof -ti:8787 | xargs kill -9`, or set `PORT=8788` in `.env`.

**`/admin/init-db` fails with a permissions error** — DSQL doesn't allow `CREATE SCHEMA` until your IAM principal has `dsql:DbConnectAdmin` (not just `dsql:DbConnect`).

---

## Next Steps

- [API.md](./API.md) — all 22 endpoints with curl examples
- [ARCHITECTURE.md](./ARCHITECTURE.md) — request flow, state, phases, scaling
- [DEVELOPMENT.md](./DEVELOPMENT.md) — code style, adding features, debugging
- [INSTALLATION.md](./INSTALLATION.md) — AWS setup, Docker, Render deployment
- [../docs/dynamo.md](../docs/dynamo.md) — DynamoDB schema and TTL
- [../docs/airflow-integration.md](../docs/airflow-integration.md) — example DAGs

---

## Project Info

- **Language:** Rust 1.80+ (edition 2021)
- **Framework:** actix-web 4
- **Database:** Amazon Aurora DSQL (PostgreSQL-compatible) + Amazon DynamoDB
- **Authentication:** JWT (HS256) + bcrypt
- **API docs:** Swagger UI, auto-generated via utoipa

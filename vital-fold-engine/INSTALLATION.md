# Installation & Deployment Guide

Complete guide for installing and deploying VitalFold Engine. The engine targets **Amazon Aurora DSQL** (PostgreSQL-compatible) plus **Amazon DynamoDB**; plain PostgreSQL is not supported at runtime.

## Table of Contents

1. [Local Development Setup](#local-development-setup)
2. [AWS Prerequisites](#aws-prerequisites)
3. [Environment Configuration](#environment-configuration)
4. [Render.com Deployment](#rendercom-deployment)
5. [Docker Deployment](#docker-deployment)
6. [Troubleshooting](#troubleshooting)
7. [Upgrade & Maintenance](#upgrade--maintenance)

---

## Local Development Setup

### Prerequisites

- **Rust 1.80+** — install from [rustup.rs](https://rustup.rs/)
- **Aurora DSQL cluster** + two DynamoDB tables (see [AWS Prerequisites](#aws-prerequisites))
- **AWS credentials** with `dsql:DbConnectAdmin` and DynamoDB write permissions
- **Git**, **curl**, **jq**

### Step 1 — Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustc --version
cargo --version
```

### Step 2 — Clone & Build

```bash
git clone <repository-url>
cd vital-fold-engine
cargo build --release
```

### Step 3 — Configure Environment

```bash
cp .env.example .env
# Fill in DSQL_CLUSTER_ENDPOINT, JWT_SECRET, ADMIN_USERNAME, ADMIN_PASSWORD
```

See [Environment Configuration](#environment-configuration) below for the full list.

### Step 4 — Run

```bash
RUST_LOG=debug cargo run --release
# Server listens on 0.0.0.0:8787
```

### Step 5 — Initialize the Schema

The engine creates all tables via a single admin endpoint that executes `migrations/init.sql`. There is no `cargo sqlx migrate` step.

```bash
TOKEN=$(curl -s -X POST http://127.0.0.1:8787/api/v1/auth/admin-login \
  -H 'Content-Type: application/json' \
  -d "{\"username\":\"$ADMIN_USERNAME\",\"password\":\"$ADMIN_PASSWORD\"}" \
  | jq -r .token)

curl -X POST http://127.0.0.1:8787/admin/init-db \
  -H "Authorization: Bearer $TOKEN"
```

Or click **Init Database** on the admin dashboard at `http://127.0.0.1:8787/`.

### Step 6 — Test

```bash
curl http://127.0.0.1:8787/health
open http://127.0.0.1:8787/swagger-ui/
```

---

## AWS Prerequisites

VitalFold Engine needs three AWS resources: an **Aurora DSQL cluster**, **two DynamoDB tables**, and an **IAM principal** with permissions to both.

> **Note:** Amazon Aurora DSQL (launched 2024) is a different product from classic Aurora Serverless. It has its own console, uses IAM-based authentication, and is serverless by default with no VPC/security-group configuration required. Do not confuse it with "Aurora with PostgreSQL compatibility".

### Step 1 — Create an Aurora DSQL Cluster

1. Open the [AWS Console → Aurora DSQL](https://console.aws.amazon.com/dsql/).
2. Click **Create cluster** and pick a region (e.g. `us-east-2`).
3. Enable or disable multi-region according to your needs (single-region is fine for development).
4. Wait until status shows **Active**.
5. Copy the **cluster endpoint** — it will look like `<cluster-id>.dsql.<region>.on.aws`.

Or via CLI:

```bash
aws dsql create-cluster \
  --region us-east-2 \
  --deletion-protection-enabled false \
  --tags Key=Project,Value=vital-fold-engine
```

### Step 2 — Create DynamoDB Tables

The app writes to two on-demand DynamoDB tables during `POST /simulate/date-range`. Create both in the same region as your DSQL cluster:

```bash
aws dynamodb create-table \
  --region us-east-2 \
  --table-name patient_visit \
  --attribute-definitions \
      AttributeName=patient_id,AttributeType=S \
      AttributeName=sort_key,AttributeType=S \
  --key-schema \
      AttributeName=patient_id,KeyType=HASH \
      AttributeName=sort_key,KeyType=RANGE \
  --billing-mode PAY_PER_REQUEST

aws dynamodb create-table \
  --region us-east-2 \
  --table-name patient_vitals \
  --attribute-definitions \
      AttributeName=patient_visit_id,AttributeType=S \
  --key-schema \
      AttributeName=patient_visit_id,KeyType=HASH \
  --billing-mode PAY_PER_REQUEST
```

See [`docs/dynamo.md`](../docs/dynamo.md) for the canonical key shape, item structure, and TTL configuration.

### Step 3 — Create an IAM Principal

The application authenticates to Aurora DSQL using an IAM-signed auth token and writes to DynamoDB with the same credentials. Create a user or role with a minimal policy:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "DsqlAdminConnect",
      "Effect": "Allow",
      "Action": [
        "dsql:DbConnect",
        "dsql:DbConnectAdmin"
      ],
      "Resource": "arn:aws:dsql:us-east-2:<your-account-id>:cluster/<cluster-id>"
    },
    {
      "Sid": "DynamoDbWrite",
      "Effect": "Allow",
      "Action": [
        "dynamodb:PutItem",
        "dynamodb:BatchWriteItem",
        "dynamodb:DeleteItem",
        "dynamodb:Scan",
        "dynamodb:Query",
        "dynamodb:DescribeTable"
      ],
      "Resource": [
        "arn:aws:dynamodb:us-east-2:<your-account-id>:table/patient_visit",
        "arn:aws:dynamodb:us-east-2:<your-account-id>:table/patient_vitals"
      ]
    }
  ]
}
```

For a quick local setup you can attach this policy to an IAM user and put the access key ID + secret key in `.env`. For production, prefer an IAM role assumed via instance profile, ECS task role, or IRSA.

Refer to the [AWS Aurora DSQL IAM docs](https://docs.aws.amazon.com/aurora-dsql/latest/userguide/accessing-sql-access-iam.html) for the current authoritative permission names — AWS occasionally renames IAM actions for new services.

### Step 4 — Generate a JWT Secret

The API signs tokens with `JWT_SECRET`. Generate a strong one:

```bash
openssl rand -base64 32
```

Paste the result into `.env` as `JWT_SECRET=...`.

### Step 5 — Initialize the Schema

See [Local Development Setup → Step 5](#step-5--initialize-the-schema) above — the same `POST /admin/init-db` call creates all 16 `vital_fold.*` tables plus `public.users`.

### Cost Guidance

Aurora DSQL and DynamoDB on-demand are both serverless — you pay per request.

| Workload | Rough monthly cost |
|---|---|
| Dev / demo (occasional populate runs) | < $5 |
| Weekly 30-day populate + daily sync | $20–60 |
| Continuous integration / nightly runs | $50–150 |

---

## Environment Configuration

All runtime config is loaded from `.env` via `dotenvy`. The [`.env.example`](.env.example) file in the repo is the authoritative template — copy it and fill in your values.

### Supported Variables

| Variable | Required? | Default | Purpose |
|---|---|---|---|
| `DSQL_CLUSTER_ENDPOINT` | **required** | — | Aurora DSQL cluster hostname, e.g. `xxx.dsql.us-east-1.on.aws` |
| `JWT_SECRET` | **required** | — | HS256 signing key, ≥32 characters |
| `HOST` | optional | `0.0.0.0` | Bind address |
| `PORT` | optional | `8787` | HTTP port |
| `DSQL_REGION` | optional | `us-east-1` | AWS region of the DSQL cluster |
| `DSQL_DB_NAME` | optional | `postgres` | Logical database name |
| `DSQL_USER` | optional | `admin` | DSQL user for IAM auth |
| `DB_POOL_SIZE` | optional | `10` | Max connections in the sqlx pool |
| `JWT_EXPIRY_HOURS` | optional | `24` | Token lifetime |
| `ADMIN_USERNAME` | optional | — | Enables `POST /api/v1/auth/admin-login` when both admin vars are set |
| `ADMIN_PASSWORD` | optional | — | Admin password (plain text via env var) |
| `STATIC_DIR` | optional | compiled-in | Override path to the Preact admin UI static files |
| `RUST_LOG` | optional | `info` | `tracing-subscriber` filter |
| `AWS_*` | optional | credential chain | Standard AWS credential env vars — used if no role/profile is available |

There is no `DATABASE_URL` at runtime. A commented `DATABASE_URL` appears in `.env.example` only as a hint for developers who want to enable sqlx compile-time query checking; it is not read by the server.

### Environment Profiles

**Development**
```env
HOST=127.0.0.1
PORT=8787
DB_POOL_SIZE=5
JWT_EXPIRY_HOURS=8
RUST_LOG=vital_fold_engine=debug,actix_web=debug,sqlx=warn
```

**Production**
```env
HOST=0.0.0.0
PORT=8787
DB_POOL_SIZE=20
JWT_EXPIRY_HOURS=24
RUST_LOG=vital_fold_engine=info,actix_web=warn
# Use AWS Secrets Manager for JWT_SECRET and ADMIN_PASSWORD
```

---

## Render.com Deployment

### Step 1 — Push to GitHub

```bash
git init
git add .
git commit -m "Initial commit: VitalFold Engine"
git remote add origin <your-github-url>
git push -u origin main
```

### Step 2 — Create Render Service

1. Go to [https://render.com](https://render.com).
2. Click **New +** → **Web Service**.
3. Connect GitHub and select the repository.
4. Configure:

   | Field | Value |
   |-------|-------|
   | **Name** | vital-fold-engine |
   | **Environment** | Rust |
   | **Build Command** | `cargo build --release` |
   | **Start Command** | `./target/release/vital-fold-engine` |
   | **Instance Type** | Standard (for production) |

### Step 3 — Configure Environment Variables

In Render dashboard → Environment:

```
HOST=0.0.0.0
PORT=3000
RUST_LOG=vital_fold_engine=info,actix_web=warn
JWT_SECRET=<generate-strong-secret>
JWT_EXPIRY_HOURS=24

DSQL_CLUSTER_ENDPOINT=<your-cluster-id>.dsql.us-east-2.on.aws
DSQL_REGION=us-east-2
DSQL_DB_NAME=postgres
DSQL_USER=admin
DB_POOL_SIZE=10

AWS_ACCESS_KEY_ID=<from-IAM>
AWS_SECRET_ACCESS_KEY=<from-IAM>

ADMIN_USERNAME=admin
ADMIN_PASSWORD=<strong-password>
```

### Step 4 — Deploy

1. Click **Deploy**.
2. Watch build logs.
3. Once live, test:

```bash
curl https://vital-fold-engine.onrender.com/health
open https://vital-fold-engine.onrender.com/swagger-ui/
```

### Render Limitations & Workarounds

| Issue | Solution |
|-------|----------|
| Free tier sleeps after 15 min inactivity | Upgrade to Starter tier |
| Build timeout (15 min limit) | Cache dependencies (Render does this automatically) |
| Cold starts | Use Starter tier or higher |

---

## Docker Deployment

### Dockerfile

Create `Dockerfile` in project root:

```dockerfile
# Build stage
FROM rust:latest as builder

WORKDIR /app
COPY . .

RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/vital-fold-engine .
COPY --from=builder /app/vital-fold-engine/static ./static

EXPOSE 8787

CMD ["./vital-fold-engine"]
```

### Build & Run

```bash
docker build -t vital-fold-engine:latest .

docker run -d \
  --name vital-fold \
  -p 8787:8787 \
  -e DSQL_CLUSTER_ENDPOINT="<your-cluster-id>.dsql.us-east-2.on.aws" \
  -e DSQL_REGION="us-east-2" \
  -e JWT_SECRET="<your-32-char-secret>" \
  -e ADMIN_USERNAME="admin" \
  -e ADMIN_PASSWORD="<strong-password>" \
  -e AWS_ACCESS_KEY_ID="<from-IAM>" \
  -e AWS_SECRET_ACCESS_KEY="<from-IAM>" \
  -e AWS_REGION="us-east-2" \
  vital-fold-engine:latest

docker logs -f vital-fold
docker stop vital-fold
```

### Docker Compose

VitalFold has no database sidecar — it talks to Aurora DSQL directly. The compose file is just the app service plus any helpers you need:

```yaml
version: '3.8'

services:
  app:
    build: .
    environment:
      DSQL_CLUSTER_ENDPOINT: ${DSQL_CLUSTER_ENDPOINT}
      DSQL_REGION: ${DSQL_REGION:-us-east-2}
      DSQL_DB_NAME: postgres
      DSQL_USER: admin
      JWT_SECRET: ${JWT_SECRET}
      ADMIN_USERNAME: ${ADMIN_USERNAME}
      ADMIN_PASSWORD: ${ADMIN_PASSWORD}
      AWS_ACCESS_KEY_ID: ${AWS_ACCESS_KEY_ID}
      AWS_SECRET_ACCESS_KEY: ${AWS_SECRET_ACCESS_KEY}
      AWS_REGION: ${DSQL_REGION:-us-east-2}
      RUST_LOG: vital_fold_engine=info,actix_web=warn
      HOST: 0.0.0.0
    ports:
      - "8787:8787"
```

Run with `docker-compose up -d` after exporting the required env vars.

---

## Troubleshooting

### `Failed to load configuration from environment`

Either `DSQL_CLUSTER_ENDPOINT` or `JWT_SECRET` is missing, or `JWT_SECRET` is shorter than 32 characters. Confirm with:

```bash
grep -E '^DSQL_CLUSTER_ENDPOINT|^JWT_SECRET' .env
```

### `Failed to create database pool`

The sqlx pool could not establish a DSQL connection on startup. Common causes:

```bash
# Verify AWS credentials are visible to the process
aws sts get-caller-identity

# Verify the IAM policy attaches dsql:DbConnectAdmin to the cluster ARN
aws iam get-user-policy --user-name <user> --policy-name <policy>

# Verify the cluster is ACTIVE in the correct region
aws dsql get-cluster --region us-east-2 --identifier <cluster-id>
```

### `/admin/init-db` returns 500 / permission error

`CREATE SCHEMA vital_fold` requires `dsql:DbConnectAdmin`. If your principal only has `dsql:DbConnect`, upgrade the policy (see [AWS Prerequisites → Step 3](#step-3--create-an-iam-principal)).

### `Port 8787 already in use`

```bash
lsof -ti:8787 | xargs kill -9
# Or change the port
echo "PORT=8788" >> .env
```

### IAM token refresh errors in logs

DSQL IAM tokens expire every ~15 minutes. The engine refreshes them automatically via a background task (see [src/db/mod.rs](src/db/mod.rs)). If you see repeated refresh failures:

- Check that the process can still reach the STS endpoint.
- Check the system clock — STS signature validation is strict about skew.

### JWT Token Errors

```bash
# Rotate the secret
openssl rand -base64 32
# Update .env, restart the server
```

### Debug Logging

```bash
# High verbosity
RUST_LOG=vital_fold_engine=debug,actix_web=debug,sqlx=debug cargo run --release

# Trace-level for one module
RUST_LOG=vital_fold_engine::generators=trace cargo run --release
```

---

## Upgrade & Maintenance

### Update Dependencies

```bash
cargo outdated   # requires `cargo install cargo-outdated`
cargo update
cargo test --all-targets
```

### Backup & Restore

Aurora DSQL provides managed automated backups and point-in-time recovery; use the AWS console or CLI (`aws dsql` commands) to manage snapshots. DynamoDB on-demand backups are configured per-table.

The synthetic data in this system is regeneratable end-to-end via the populate endpoints, so backups are rarely load-bearing — the dataset is meant to be rebuilt whenever needed.

### Zero-Downtime Deployment

1. Deploy the new binary to a staging instance.
2. Run `cargo test --all-targets` against the build.
3. Deploy to production during low-traffic window.
4. Monitor `/simulate/status` and Render/ECS metrics for errors.

---

## Next Steps

- [API.md](./API.md) — all 22 endpoints with curl examples
- [DEVELOPMENT.md](./DEVELOPMENT.md) — code style and development workflow
- [ARCHITECTURE.md](./ARCHITECTURE.md) — system design details
- [../docs/dynamo.md](../docs/dynamo.md) — DynamoDB item shape and TTL
- [../docs/airflow-integration.md](../docs/airflow-integration.md) — orchestration examples

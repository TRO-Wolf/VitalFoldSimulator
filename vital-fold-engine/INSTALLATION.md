# Installation & Deployment Guide

Complete guide for installing and deploying VitalFold Engine in various environments.

## Table of Contents

1. [Local Development Setup](#local-development-setup)
2. [Database Setup](#database-setup)
3. [Environment Configuration](#environment-configuration)
4. [Render.com Deployment](#rendercom-deployment)
5. [Docker Deployment](#docker-deployment)
6. [Aurora DSQL Setup](#aurora-dsql-setup)
7. [Troubleshooting](#troubleshooting)

---

## Local Development Setup

### Prerequisites

- **Rust 1.80+** (install from [rustup.rs](https://rustup.rs/))
- **PostgreSQL 14+** OR **Aurora DSQL** connection
- **Git** for version control
- **curl** for API testing

### Step 1: Install Rust

```bash
# Download and install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add Rust to PATH
source $HOME/.cargo/env

# Verify installation
rustc --version
cargo --version
```

### Step 2: Clone Repository

```bash
git clone <repository-url>
cd vital-fold-engine
```

### Step 3: Install Dependencies

```bash
# Build all dependencies (this takes 2-3 minutes on first build)
cargo build

# Verify no compilation errors
cargo check
```

### Step 4: Configure Environment

```bash
# Copy example configuration
cp .env.example .env

# Edit with your database details
nano .env
```

### Step 5: Setup Database

```bash
# Run migrations
cargo sqlx migrate run

# Verify tables were created
psql -h localhost -U postgres -d postgres -c "\dt"
```

### Step 6: Run Locally

```bash
# Start development server with debug logging
RUST_LOG=debug cargo run

# Server starts on http://127.0.0.1:8787
```

### Step 7: Test API

```bash
# Health check
curl http://127.0.0.1:8787/health

# View Swagger UI
open http://127.0.0.1:8787/swagger-ui/
```

---

## Database Setup

### Local PostgreSQL

#### macOS (with Homebrew)

```bash
# Install PostgreSQL
brew install postgresql

# Start PostgreSQL service
brew services start postgresql

# Create database
createdb vital_fold_db

# Create user
createuser -P vital_user
# When prompted, enter password

# Grant privileges
psql -d vital_fold_db -c "GRANT ALL PRIVILEGES ON DATABASE vital_fold_db TO vital_user;"
```

#### Ubuntu/Debian

```bash
# Install PostgreSQL
sudo apt-get update
sudo apt-get install postgresql postgresql-contrib

# Start service
sudo systemctl start postgresql

# Create database and user
sudo -u postgres createdb vital_fold_db
sudo -u postgres createuser -P vital_user
sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE vital_fold_db TO vital_user;"
```

#### Windows (with PostgreSQL Installer)

1. Download [PostgreSQL for Windows](https://www.postgresql.org/download/windows/)
2. Run installer, accept defaults
3. Remember the password for postgres user
4. Use pgAdmin GUI or command line:
```bash
createdb -U postgres vital_fold_db
createuser -U postgres -P vital_user
```

### Connection String

Update `.env` with your PostgreSQL connection:

```env
DATABASE_URL=postgres://vital_user:password@localhost:5432/vital_fold_db
```

---

## Environment Configuration

### .env File Template

```env
# ─── Server ────────────────────────────────────────────────────────────────
HOST=127.0.0.1
PORT=8787

# ─── Database: PostgreSQL (Local Development) ───────────────────────────────
# For local PostgreSQL
DATABASE_URL=postgres://vital_user:password@localhost:5432/vital_fold_db
DB_POOL_SIZE=10

# ─── Database: Aurora DSQL (AWS Production) ────────────────────────────────
# Uncomment and configure for Aurora DSQL
# DSQL_ENDPOINT=cluster-name.dsql.region.on.aws
# DSQL_CLUSTER_ENDPOINT=cluster-name.dsql.region.on.aws
# DSQL_REGION=us-east-2
# DSQL_DB_NAME=postgres
# DSQL_USER=admin
# DSQL_PORT=5432

# ─── AWS Credentials (for DSQL and other AWS services) ──────────────────────
# Get from AWS IAM console
# ⚠️  NEVER commit real credentials - use Secrets Manager in production
AWS_REGION=us-east-2
AWS_ACCESS_KEY_ID=AKIA...
AWS_SECRET_ACCESS_KEY=...

# ─── Authentication & Security ─────────────────────────────────────────────
# Generate with: openssl rand -base64 32
JWT_SECRET=your-secret-key-must-be-at-least-32-characters-long-for-production
JWT_EXPIRY_HOURS=24

# ─── Logging ───────────────────────────────────────────────────────────────
# Set to 'debug' for development, 'info' for production
RUST_LOG=vital_fold_engine=debug,actix_web=debug

# ─── SQLx Offline Mode (optional) ──────────────────────────────────────────
# For CI/CD pipelines without database access
# SQLX_OFFLINE=true
```

### Development vs Production

**Development Environment**
```env
HOST=127.0.0.1
PORT=8787
DB_POOL_SIZE=5
JWT_EXPIRY_HOURS=8
RUST_LOG=debug
```

**Staging Environment**
```env
HOST=0.0.0.0
PORT=8787
DB_POOL_SIZE=15
JWT_EXPIRY_HOURS=24
RUST_LOG=info
```

**Production Environment**
```env
HOST=0.0.0.0
PORT=8787
DB_POOL_SIZE=20
JWT_EXPIRY_HOURS=24
RUST_LOG=info,actix_web=warn
# Use AWS Secrets Manager for credentials
```

---

## Render.com Deployment

### Step 1: Push to GitHub

```bash
# Initialize git repo (if not already done)
git init
git add .
git commit -m "Initial commit: VitalFold Engine"
git remote add origin <your-github-url>
git push -u origin main
```

### Step 2: Create Render Service

1. Go to [https://render.com](https://render.com)
2. Sign in or create account
3. Click "New +" → "Web Service"
4. Connect GitHub account and select repository
5. Configure service:

   | Field | Value |
   |-------|-------|
   | **Name** | vital-fold-engine |
   | **Environment** | Rust |
   | **Build Command** | `cargo build --release` |
   | **Start Command** | `./target/release/vital-fold-engine` |
   | **Instance Type** | Standard (for production) |

### Step 3: Configure Environment Variables

In Render dashboard → Environment:

```
HOST=0.0.0.0
PORT=3000
RUST_LOG=info,actix_web=warn
JWT_SECRET=<generate-strong-secret>
JWT_EXPIRY_HOURS=24

# Database (configure based on your setup)
DATABASE_URL=<your-database-connection-string>
DB_POOL_SIZE=10

# AWS credentials (if using DSQL)
AWS_REGION=us-east-2
AWS_ACCESS_KEY_ID=<from-IAM>
AWS_SECRET_ACCESS_KEY=<from-IAM>
```

### Step 4: Connect Database

#### Option A: Use Aurora DSQL
1. Configure `DSQL_*` environment variables in Render
2. Ensure DSQL cluster allows connections from Render IPs

#### Option B: Use Render PostgreSQL
1. Create Render PostgreSQL instance
2. Note connection string
3. Set as `DATABASE_URL` environment variable
4. Render will run migrations automatically

### Step 5: Deploy

1. Click "Deploy" button
2. Watch build logs
3. Once deployed, you'll get a URL like `https://vital-fold-engine.onrender.com`

### Step 6: Post-Deployment

```bash
# Test deployed API
curl https://vital-fold-engine.onrender.com/health

# Access Swagger UI
open https://vital-fold-engine.onrender.com/swagger-ui/
```

### Render.com Limitations & Workarounds

| Issue | Solution |
|-------|----------|
| Free tier sleeps after 15 min inactivity | Upgrade to Starter tier |
| Database connection pooling limits | Use connection pooler like PgBouncer |
| Build time out (15 min limit) | Optimize dependencies |
| Cold starts | Use Starter tier or higher |

---

## Docker Deployment

### Build Docker Image

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

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/vital-fold-engine .

EXPOSE 8787

CMD ["./vital-fold-engine"]
```

### Build & Run

```bash
# Build image
docker build -t vital-fold-engine:latest .

# Run container
docker run -d \
  --name vital-fold \
  -p 8787:8787 \
  -e DATABASE_URL="postgres://user:pass@db:5432/vital_fold_db" \
  -e JWT_SECRET="your-secret-key" \
  vital-fold-engine:latest

# Check logs
docker logs vital-fold

# Stop container
docker stop vital-fold
```

### Docker Compose

Create `docker-compose.yml`:

```yaml
version: '3.8'

services:
  postgres:
    image: postgres:15
    environment:
      POSTGRES_DB: vital_fold_db
      POSTGRES_USER: vital_user
      POSTGRES_PASSWORD: secure_password
    volumes:
      - postgres_data:/var/lib/postgresql/data
    ports:
      - "5432:5432"
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U vital_user"]
      interval: 10s
      timeout: 5s
      retries: 5

  app:
    build: .
    environment:
      DATABASE_URL: postgres://vital_user:secure_password@postgres:5432/vital_fold_db
      JWT_SECRET: your-secret-key
      RUST_LOG: info
      HOST: 0.0.0.0
    ports:
      - "8787:8787"
    depends_on:
      postgres:
        condition: service_healthy

volumes:
  postgres_data:
```

Run with:

```bash
docker-compose up -d
```

---

## Aurora DSQL Setup

### Prerequisites

- AWS Account
- IAM credentials with DSQL permissions
- VPC configured (or use default)

### Step 1: Create Aurora DSQL Cluster

1. Go to [AWS Console → Amazon Aurora](https://console.aws.amazon.com/rds/)
2. Click "Create Database"
3. Select "Amazon Aurora with PostgreSQL compatibility"
4. Choose "Serverless v2"
5. Configure:
   - **Cluster identifier**: `vital-fold-cluster`
   - **Master username**: `admin`
   - **Master password**: Use strong password or AWS Secrets Manager
   - **Database name**: `postgres`
   - **Backup retention**: 7 days (production minimum)
   - **Region**: Your preferred region
   - **VPC**: Select appropriate VPC

### Step 2: Configure Security Groups

1. In RDS console, find your cluster
2. Click "Modify"
3. Under "Security and network", edit security group
4. Add inbound rule:
   - Type: PostgreSQL
   - Port: 5432
   - Source: Your application's IP or security group

### Step 3: Get Connection String

1. In RDS console, find your cluster
2. Click "Connectivity & security"
3. Copy Writer endpoint (e.g., `cluster-name.dsql.us-east-2.on.aws`)

### Step 4: Configure Application

Update `.env`:

```env
DSQL_ENDPOINT=cluster-name.dsql.us-east-2.on.aws
DSQL_CLUSTER_ENDPOINT=cluster-name.dsql.us-east-2.on.aws
DSQL_REGION=us-east-2
DSQL_DB_NAME=postgres
DSQL_USER=admin
DSQL_PORT=5432
AWS_REGION=us-east-2
AWS_ACCESS_KEY_ID=<from-IAM>
AWS_SECRET_ACCESS_KEY=<from-IAM>
```

### Step 5: Run Migrations

```bash
cargo sqlx migrate run
```

### Cost Optimization

Aurora DSQL is serverless and auto-scales. Typical monthly costs:

| Workload | Estimated Cost |
|----------|---|
| Development (low usage) | $1-5/month |
| Small production | $20-50/month |
| Large production | $100-500/month |

---

## Troubleshooting

### Common Issues

#### "Connection refused" to Database

```bash
# Verify database is running
psql -h localhost -U postgres -c "SELECT 1"

# Check DATABASE_URL in .env
cat .env | grep DATABASE_URL

# For Aurora DSQL, verify:
# 1. Cluster is running
# 2. Security group allows your IP
# 3. Credentials are correct
```

#### "Address already in use" on Port 8787

```bash
# Find process using port
lsof -i :8787

# Kill process
kill -9 <PID>

# Or change port in .env
echo "PORT=8788" >> .env
```

#### Slow Build on First Run

```bash
# First build caches dependencies (2-3 min)
cargo build

# Subsequent builds are faster (30-60 sec)
cargo build

# Release builds take longer but run faster
cargo build --release  # ~5 min first time
```

#### Migration Failures

```bash
# Check migration status
cargo sqlx migrate list

# Revert last migration
cargo sqlx migrate revert

# Create new migration
cargo sqlx migrate add -r migration_name

# Verify schema
psql -h localhost -U postgres -d vital_fold_db -c "\dt"
```

#### JWT Token Errors

```bash
# Generate new JWT secret
openssl rand -base64 32

# Update .env
echo "JWT_SECRET=<new-secret>" >> .env

# Restart server
cargo run
```

### Debug Mode

Enable debug logging:

```bash
# High verbosity
RUST_LOG=debug,actix_web=debug cargo run

# Specific module
RUST_LOG=vital_fold_engine=trace cargo run

# With timestamps
RUST_LOG=debug cargo run 2>&1 | tee app.log
```

### Performance Debugging

```bash
# Check database pool stats (enable in code)
# See application logs for connection pool status

# Monitor system resources
top -p $(pgrep -f "vital-fold-engine")

# Check database connections
psql -h localhost -U postgres -c \
  "SELECT count(*) FROM pg_stat_activity WHERE datname='vital_fold_db';"
```

### Getting Help

1. Check server logs: `RUST_LOG=debug cargo run`
2. Test endpoint directly: `curl -v http://127.0.0.1:8787/health`
3. Check Swagger UI for endpoint documentation
4. Review error responses for detailed error messages

---

## Upgrade & Maintenance

### Update Dependencies

```bash
# Check for outdated packages
cargo outdated

# Update all dependencies
cargo update

# Update specific package
cargo update -p actix-web

# Run tests after update
cargo test
```

### Backup Database

#### PostgreSQL Local

```bash
# Full database backup
pg_dump -U postgres vital_fold_db > backup.sql

# Restore
psql -U postgres vital_fold_db < backup.sql
```

#### Aurora DSQL

1. AWS RDS console → Automated backups
2. Manual snapshots for important versions
3. Point-in-time recovery available up to retention period

### Zero-Downtime Deployment

1. Deploy new version to staging environment
2. Run full test suite
3. Create database backup
4. Deploy to production during low-traffic window
5. Monitor error rates and performance

---

## Next Steps

- Review [API.md](./API.md) for API documentation
- See [DEVELOPMENT.md](./DEVELOPMENT.md) for development workflow
- Check [ARCHITECTURE.md](./ARCHITECTURE.md) for system design details


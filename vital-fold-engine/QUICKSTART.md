# Quick Start Guide

Get VitalFold Engine running in 5 minutes.

## 1. Prerequisites

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Verify installation
rustc --version  # Should be 1.80+
cargo --version
```

## 2. Clone & Setup

```bash
# Clone repository
git clone <your-repo-url>
cd vital-fold-engine

# Copy environment template
cp .env.example .env

# Edit .env with your database URL
nano .env
# Or use your preferred editor
```

## 3. Setup Database

### Local PostgreSQL
```bash
# Create database
createdb vital_fold_db

# Run migrations
cargo sqlx migrate run
```

### OR Aurora DSQL
Update `.env`:
```env
DSQL_ENDPOINT=your-cluster.dsql.region.on.aws
DSQL_USER=admin
DSQL_PORT=5432
```

Then run migrations:
```bash
cargo sqlx migrate run
```

## 4. Start Server

```bash
# Development with debug logs
RUST_LOG=debug cargo run

# Or just run (info logs)
cargo run

# Server ready: http://127.0.0.1:8787
```

## 5. Test API

```bash
# Health check
curl http://127.0.0.1:8787/health

# Register user
curl -X POST http://127.0.0.1:8787/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email":"test@example.com","password":"TestPassword123"}'

# Save the token from response, then test protected endpoint:
TOKEN="your-token-from-response"

curl -X GET http://127.0.0.1:8787/api/v1/me \
  -H "Authorization: Bearer $TOKEN"
```

## 6. Interactive Exploration

Open browser to:
```
http://127.0.0.1:8787/swagger-ui/
```

Click "Authorize", enter your JWT token, explore endpoints interactively.

---

## Common Commands

```bash
# Run all tests
cargo test

# Format code
cargo fmt

# Lint code
cargo clippy

# Build release binary
cargo build --release

# Clean build artifacts
cargo clean

# Update dependencies
cargo update

# View documentation
cargo doc --open
```

---

## First Simulation

```bash
TOKEN="your-jwt-token"

# Start simulation
curl -X POST http://127.0.0.1:8787/simulate \
  -H "Authorization: Bearer $TOKEN"

# Check status
curl http://127.0.0.1:8787/simulate/status \
  -H "Authorization: Bearer $TOKEN"

# Stop simulation
curl -X POST http://127.0.0.1:8787/simulate/stop \
  -H "Authorization: Bearer $TOKEN"

# Reset data
curl -X POST http://127.0.0.1:8787/simulate/reset \
  -H "Authorization: Bearer $TOKEN"
```

---

## Troubleshooting

**"Database connection refused"**
```bash
# Check PostgreSQL is running
psql -h localhost -U postgres -c "SELECT 1"

# Or check .env DATABASE_URL is correct
cat .env | grep DATABASE_URL
```

**"Port 8787 already in use"**
```bash
# Kill process using port
lsof -ti:8787 | xargs kill -9

# Or use different port
PORT=8888 cargo run
```

**"Migration failed"**
```bash
# Check migration status
cargo sqlx migrate list

# Revert and retry
cargo sqlx migrate revert
cargo sqlx migrate run
```

---

## Next Steps

- Read [README.md](./README.md) for full overview
- See [API.md](./API.md) for endpoint documentation
- Check [INSTALLATION.md](./INSTALLATION.md) for detailed setup
- Review [DEVELOPMENT.md](./DEVELOPMENT.md) for development workflow
- Study [ARCHITECTURE.md](./ARCHITECTURE.md) for technical deep-dive

---

## Key Files

| File | Purpose |
|------|---------|
| `.env` | Configuration (database, JWT secret, etc.) |
| `src/main.rs` | Application entry point |
| `src/routes.rs` | API routes |
| `src/handlers/` | Request handlers |
| `src/generators/` | Data generation logic |
| `migrations/` | Database schema |
| `Cargo.toml` | Dependencies |

---

## Project Info

- **Language**: Rust 1.80+
- **Framework**: Actix-web 4.x
- **Database**: PostgreSQL / Aurora DSQL
- **Authentication**: JWT + bcrypt
- **API Docs**: Swagger UI (auto-generated)

---

**Ready to go!** Start coding and building amazing healthcare data pipelines. 🚀

# Development Guide

Guide for developers working on VitalFold Engine codebase.

## Table of Contents

1. [Development Environment](#development-environment)
2. [Project Structure](#project-structure)
3. [Building & Running](#building--running)
4. [Testing](#testing)
5. [Code Style](#code-style)
6. [Debugging](#debugging)
7. [Adding Features](#adding-features)
8. [Database Migrations](#database-migrations)
9. [Common Tasks](#common-tasks)

---

## Development Environment

### Requirements

- **Rust 1.80+** — [Install](https://rustup.rs/)
- **Cargo** — Package manager (included with Rust)
- **PostgreSQL 14+** — Local database for development
- **Git** — Version control
- **VS Code** or **IDE** of choice

### Recommended Tools

```bash
# Rust analyzer for IDE support
rustup component add rust-analyzer

# Code formatter
rustup component add rustfmt

# Linter
rustup component add clippy

# Additional utilities
cargo install cargo-watch      # Auto-run on file changes
cargo install cargo-edit       # Add/update dependencies
cargo install cargo-tarpaulin  # Code coverage
```

### IDE Setup

#### VS Code

Install extensions:
- **Rust Analyzer** (rust-lang.rust-analyzer)
- **CodeLLDB** (vadimcn.vscode-lldb) for debugging
- **Better TOML** (bungcip.better-toml)
- **Even Better TOML** (tamasfe.even-better-toml)

#### JetBrains IntelliJ IDEA

- Install Rust plugin
- Enable Cargo run configurations
- Code inspections enabled automatically

---

## Project Structure

```
vital-fold-engine/
├── src/
│   ├── main.rs                 # Application entry point
│   ├── lib.rs                  # Library exports (if used)
│   ├── routes.rs               # Route configuration
│   ├── db.rs                   # Database connection setup
│   ├── errors.rs               # Error types
│   ├── config.rs               # Configuration management
│   ├── engine_state.rs         # Global simulation state
│   │
│   ├── handlers/               # HTTP request handlers
│   │   ├── mod.rs
│   │   ├── health.rs           # Health check endpoint
│   │   ├── auth.rs             # Authentication endpoints
│   │   ├── user.rs             # User profile endpoint
│   │   └── simulation.rs        # Simulation control endpoints
│   │
│   ├── middleware/             # Request/response middleware
│   │   ├── mod.rs
│   │   └── auth.rs             # JWT validation middleware
│   │
│   ├── generators/             # Data generation modules
│   │   ├── mod.rs              # Main generator orchestration
│   │   ├── insurance.rs        # Insurance data generation
│   │   ├── clinic.rs           # Clinic data generation
│   │   ├── provider.rs         # Provider data generation
│   │   ├── patient.rs          # Patient data generation
│   │   ├── appointment.rs      # Appointment data generation
│   │   └── medical_record.rs   # Medical record data generation
│   │
│   ├── models/                 # Data models & types
│   │   ├── mod.rs
│   │   └── user.rs             # User-related models
│   │
│   └── db/                     # Database operations
│       └── mod.rs              # Query builders
│
├── migrations/                 # SQL migrations
│   └── 001_init.sql
│
├── tests/                      # Integration tests
│   └── api_tests.rs
│
├── Cargo.toml                  # Dependencies
├── Cargo.lock                  # Lock file
├── .env                        # Environment (local dev)
├── .gitignore                  # Git ignore rules
│
├── README.md                   # Project overview
├── INSTALLATION.md             # Setup guide
├── API.md                      # API documentation
├── DEVELOPMENT.md              # This file
└── ARCHITECTURE.md             # System design

```

### Module Organization

**Handler Pattern:**

```rust
// handlers/simulation.rs
pub async fn start_simulation(
    pool: web::Data<DbPool>,
    state: web::Data<SimulatorState>,
) -> Result<HttpResponse, AppError> {
    // Implementation
}
```

**Generator Pattern:**

```rust
// generators/patient.rs
pub async fn generate_patients(ctx: &mut SimulationContext) -> Result<(), AppError> {
    // 1. Build list of patient data
    // 2. Insert into database
    // 3. Update context counts
}
```

---

## Building & Running

### Development Build

```bash
# Standard debug build (fast compile, slower runtime)
cargo build

# Verify compilation
cargo check

# Build with optimizations for testing
cargo build --release

# Clean build (remove artifacts)
cargo clean && cargo build
```

### Running

```bash
# Run with default settings
cargo run

# Run with debug logging
RUST_LOG=debug cargo run

# Run with specific module logging
RUST_LOG=vital_fold_engine=trace,actix_web=debug cargo run

# Run on specific port
PORT=8888 cargo run

# Run in background
cargo run > app.log 2>&1 &
```

### Watch Mode

Auto-reload on file changes:

```bash
# Install cargo-watch
cargo install cargo-watch

# Watch and rebuild
cargo watch -x run

# Watch and run tests
cargo watch -x test

# Watch with custom command
cargo watch -x "build --release"
```

---

## Testing

### Run Tests

```bash
# Run all tests
cargo test

# Run specific test file
cargo test --test api_tests

# Run single test
cargo test test_registration

# Run with output visible
cargo test -- --nocapture

# Run with multiple threads
cargo test -- --test-threads=4

# Run without parallelization (slower but safer)
cargo test -- --test-threads=1
```

### Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_user_login() {
        // Setup
        let pool = create_test_db().await;

        // Execute
        let result = login_user(&pool, "test@example.com", "password").await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap().email, "test@example.com");
    }
}
```

### Test Database

For integration tests:

```bash
# Create test database
createdb vital_fold_test

# Update .env.test
DATABASE_URL=postgres://user:pass@localhost:5432/vital_fold_test

# Run migrations
cargo sqlx migrate run

# Run tests
cargo test
```

### Code Coverage

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html

# View report
open tarpaulin-report.html
```

---

## Code Style

### Formatting

```bash
# Format all code
cargo fmt

# Check formatting
cargo fmt -- --check

# Format specific file
rustfmt src/handlers/auth.rs
```

### Linting

```bash
# Run clippy
cargo clippy

# Clippy with all warnings
cargo clippy -- -W clippy::all

# Fix warnings automatically
cargo clippy --fix
```

### Naming Conventions

```rust
// Constants: UPPER_SNAKE_CASE
const MAX_CONNECTIONS: u32 = 100;

// Structs: PascalCase
struct UserProfile {
    email: String,
}

// Functions: snake_case
fn generate_insurance() {
    // ...
}

// Variables: snake_case
let user_email = "user@example.com";

// Type Parameters: PascalCase
pub fn process<T: Serialize>(item: T) {
    // ...
}
```

### Error Handling

Use the custom `AppError` type:

```rust
// Good
fn get_user(id: Uuid) -> Result<User, AppError> {
    user_exists(id)
        .map_err(|_| AppError::NotFound("User not found".to_string()))?
}

// Bad - Avoid panics
fn get_user(id: Uuid) -> User {
    users.iter().find(|u| u.id == id).unwrap() // NEVER
}
```

### Comments

```rust
// Document public functions
/// Generates insurance companies for the simulation.
///
/// Creates 7 major US insurance providers in the database.
///
/// # Arguments
/// * `ctx` - Simulation context with database connection
///
/// # Returns
/// Result with error if insertion fails
pub async fn generate_insurance_companies(ctx: &mut SimulationContext) -> Result<(), AppError> {
    // Implementation note: Fixed to 7 companies
    // This matches the healthcare industry structure
}
```

---

## Debugging

### Logging

Enable structured logging:

```bash
# Debug level
RUST_LOG=debug cargo run

# Trace level (verbose)
RUST_LOG=trace cargo run

# Specific module
RUST_LOG=vital_fold_engine::generators=trace cargo run

# Multiple modules
RUST_LOG=vital_fold_engine=debug,sqlx=info cargo run

# With file output
RUST_LOG=debug cargo run > debug.log 2>&1
```

### Logging in Code

```rust
use tracing::{info, debug, warn, error};

#[actix_web::main]
async fn main() {
    // Application logic
    info!("Starting simulation");
    debug!("Database connected");
    warn!("Simulation already running");
    error!("Database connection failed");
}
```

### Debugging with IDE

#### VS Code (CodeLLDB)

1. Install CodeLLDB extension
2. Create `.vscode/launch.json`:

```json
{
    "version": "0.2.0",
    "configurations": [
        {
            "type": "lldb",
            "request": "launch",
            "name": "Debug",
            "cargo": {
                "args": [
                    "build",
                    "--bin=vital-fold-engine",
                    "--package=vital-fold-engine"
                ],
                "filter": {
                    "name": "vital-fold-engine",
                    "kind": "bin"
                }
            },
            "args": [],
            "cwd": "${workspaceFolder}"
        }
    ]
}
```

3. Click "Run and Debug" (Ctrl+Shift+D)
4. Set breakpoints and step through code

### Database Inspection

```bash
# Connect to database
psql -h localhost -U postgres -d vital_fold_db

# List tables
\dt

# View table structure
\d users

# Run query
SELECT * FROM users;

# Exit
\q
```

### Network Inspection

```bash
# View all HTTP requests
RUST_LOG=actix_web=debug cargo run

# Use curl with verbose output
curl -v -X GET http://127.0.0.1:8787/health

# Use httpie for pretty output
http http://127.0.0.1:8787/health

# Monitor network traffic
tcpdump -i lo -n 'tcp port 8787'
```

---

## Adding Features

### Adding an Endpoint

1. **Create handler function** in `src/handlers/`:

```rust
// src/handlers/example.rs
use actix_web::{web, HttpResponse};
use crate::errors::AppError;

/// Get example data
#[utoipa::path(
    get,
    path = "/api/v1/example",
    tag = "Example",
    responses(
        (status = 200, description = "Example data retrieved", body = String),
    )
)]
pub async fn get_example() -> Result<HttpResponse, AppError> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Hello World"
    })))
}
```

2. **Register in module** (`src/handlers/mod.rs`):

```rust
pub mod example;
```

3. **Add route** in `src/routes.rs`:

```rust
cfg.route("/api/v1/example", web::get().to(example::get_example));
```

4. **Test endpoint**:

```bash
curl http://127.0.0.1:8787/api/v1/example
```

### Adding a Data Generator

1. **Create generator module** (`src/generators/example.rs`):

```rust
use crate::generators::SimulationContext;
use crate::errors::AppError;

pub async fn generate_examples(ctx: &mut SimulationContext) -> Result<(), AppError> {
    // 1. Generate data
    let examples = vec![
        // Data here
    ];

    // 2. Insert into database
    for example in examples {
        sqlx::query!("INSERT INTO examples (...) VALUES (...)")
            .execute(&ctx.pool)
            .await?;
    }

    // 3. Update counts
    ctx.counts.examples = examples.len();

    Ok(())
}
```

2. **Register in module** (`src/generators/mod.rs`):

```rust
pub mod example;
```

3. **Call in orchestration** (`src/generators/mod.rs`):

```rust
pub async fn run_simulation(...) -> Result<(), AppError> {
    // ... other generators ...
    example::generate_examples(&mut ctx).await?;
}
```

### Adding a Migration

```bash
# Create migration file
cargo sqlx migrate add -r create_examples_table

# Edit migration file
nano migrations/$(ls -t migrations | head -1)

# Run migration
cargo sqlx migrate run

# Revert if needed
cargo sqlx migrate revert
```

---

## Database Migrations

### Creating Migrations

```bash
# Create new migration
cargo sqlx migrate add create_users_table

# Create reversible migration (with UP and DOWN)
cargo sqlx migrate add -r create_users_table
```

### Migration File Format

```sql
-- migrations/001_create_users.sql
-- Create users table
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create index for performance
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);

-- Add reversible section
-- ----- Down -----
-- DROP INDEX IF EXISTS idx_users_email;
-- DROP TABLE IF EXISTS users;
```

### Running Migrations

```bash
# Run all pending migrations
cargo sqlx migrate run

# Revert last migration
cargo sqlx migrate revert

# Check migration status
cargo sqlx migrate list
```

---

## Common Tasks

### Updating Dependencies

```bash
# Check outdated packages
cargo outdated

# Update specific package
cargo update -p actix-web

# Update all packages
cargo update

# Run tests after update
cargo test
```

### Building Release Binary

```bash
# Build optimized binary
cargo build --release

# Binary location
./target/release/vital-fold-engine

# Binary size
ls -lh target/release/vital-fold-engine
```

### Performance Profiling

```bash
# Install profiler
cargo install flamegraph

# Generate flame graph
cargo flamegraph

# View result
open flamegraph.svg
```

### Generating API Documentation

```bash
# Build documentation
cargo doc --open

# Documentation for dependencies
cargo doc --open --document-private-items
```

### Running Benchmarks

```bash
# Create benchmark
cargo bench

# Run specific benchmark
cargo bench simulation_generation
```

### Git Workflow

```bash
# Create feature branch
git checkout -b feature/my-feature

# Make changes
# Commit frequently
git add .
git commit -m "Add feature description"

# Push to GitHub
git push origin feature/my-feature

# Create pull request on GitHub
# After review, merge to main
```

### Security Scanning

```bash
# Audit dependencies for vulnerabilities
cargo audit

# Fix vulnerabilities
cargo audit --fix

# Check licenses
cargo license
```

---

## Troubleshooting

### Common Build Errors

**Error: "cannot find function in this scope"**
```bash
# Solution: Ensure module is properly exported
# In module/mod.rs, add:
pub mod submodule;
pub use submodule::function;
```

**Error: "lifetime mismatch"**
```bash
# Solution: Add explicit lifetime annotations
fn process<'a>(input: &'a str) -> &'a str {
    input
}
```

**Error: "type mismatch"**
```bash
# Solution: Ensure type conversions are explicit
let string_val: String = "hello".to_string();
```

### Runtime Issues

**Port already in use**
```bash
# Kill process
lsof -ti:8787 | xargs kill -9

# Or use different port
PORT=8888 cargo run
```

**Database connection fails**
```bash
# Check PostgreSQL is running
psql -h localhost -U postgres -c "SELECT 1"

# Check DATABASE_URL
echo $DATABASE_URL

# Recreate test database
dropdb vital_fold_db
createdb vital_fold_db
cargo sqlx migrate run
```

**Tests hang**
```bash
# Increase test timeout
cargo test -- --test-threads=1 --nocapture

# Or kill and restart
Ctrl+C
cargo clean && cargo test
```

### Performance Issues

```bash
# Profile with release build
cargo run --release

# Check database query performance
psql -h localhost -d vital_fold_db
EXPLAIN ANALYZE SELECT * FROM users;

# Monitor resource usage
top -p $(pgrep -f "vital-fold-engine")
```

---

## Next Steps

- Read [API.md](./API.md) for endpoint specifications
- Review [ARCHITECTURE.md](./ARCHITECTURE.md) for system design
- Check existing handlers for code patterns
- Run `cargo test` to verify your setup


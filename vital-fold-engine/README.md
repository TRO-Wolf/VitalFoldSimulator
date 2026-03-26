# VitalFold Engine

> Synthetic healthcare data generation and simulation engine for building high-quality datasets for data pipeline development and analytics.

[![Rust](https://img.shields.io/badge/Rust-1.80+-orange.svg)](https://www.rust-lang.org/)
[![Actix-web](https://img.shields.io/badge/Actix--web-4.x-success.svg)](https://actix.rs/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Status](https://img.shields.io/badge/Status-Production--Ready-brightgreen.svg)]()

## Overview

VitalFold Engine is a high-performance REST API designed to generate synthetic healthcare data based on realistic clinical schema. Built with Rust and Actix-web, it provides:

- **Scalable Data Generation**: Generate millions of synthetic patient records, appointments, medical records, and insurance relationships
- **RESTful API**: Complete API control over simulation lifecycle with authentication
- **Real-time Status Monitoring**: Track simulation progress and data counts in real-time
- **Aurora DSQL Integration**: Serverless PostgreSQL-compatible database backend
- **Interactive Documentation**: Built-in Swagger UI for API exploration and testing
- **Production-Ready**: Comprehensive error handling, logging, and security

## Key Features

✅ **Synthetic Healthcare Data Generation**
- Providers, patients, clinics, appointments, medical records
- Insurance company and plan associations
- Emergency contacts and patient demographics
- Realistic data relationships and referential integrity

✅ **REST API Control**
- Start/stop simulations on-demand
- Monitor simulation status and data metrics
- Reset generated data safely
- User authentication with JWT bearer tokens

✅ **High Performance**
- Asynchronous request handling (Tokio runtime)
- Connection pooling for database efficiency
- Concurrent data generation with task-based architecture
- Response times in milliseconds

✅ **Developer Experience**
- OpenAPI 3.0 specification with Swagger UI
- Comprehensive error messages and status codes
- Structured logging with tracing
- Configuration via environment variables

✅ **Security**
- JWT bearer token authentication on protected endpoints
- Password hashing with bcrypt
- Input validation and SQL error sanitization
- HTTPS-ready (TLS support via reverse proxy)

## Quick Start

### Prerequisites

- **Rust 1.80+** — [Install Rust](https://rustup.rs/)
- **PostgreSQL 14+** or **Aurora DSQL** — Database backend
- **Git** — Version control

### Installation

1. **Clone the repository**
```bash
git clone <repository-url>
cd vital-fold-engine
```

2. **Configure environment**
```bash
cp .env.example .env
# Edit .env with your database credentials
```

3. **Run migrations**
```bash
cargo sqlx migrate run
```

4. **Build and run**
```bash
cargo run
```

The API will be available at `http://127.0.0.1:8787`

### First Steps

1. **Health Check** (no auth required)
```bash
curl http://127.0.0.1:8787/health
```

2. **Login and Get Token**
```bash
curl -X POST http://127.0.0.1:8787/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "SecurePassword123"
  }'
```

Save the `token` from the response.

3. **Start a Simulation**
```bash
TOKEN="<your-jwt-token>"
curl -X POST http://127.0.0.1:8787/simulate \
  -H "Authorization: Bearer $TOKEN"
```

4. **Check Simulation Status**
```bash
curl http://127.0.0.1:8787/simulate/status \
  -H "Authorization: Bearer $TOKEN"
```

## API Documentation

### Interactive Documentation

Access the interactive Swagger UI when the server is running:

```
http://127.0.0.1:8787/swagger-ui/
```

Click the "Authorize" button to authenticate with your JWT token.

### OpenAPI Specification

The raw OpenAPI 3.0 specification is available at:

```
http://127.0.0.1:8787/api-docs/openapi.json
```

### Core Endpoints

#### Public Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Health check |
| POST | `/api/v1/auth/login` | Login and get JWT token |
| POST | `/api/v1/auth/admin-login` | Admin login with env credentials |

#### Protected Endpoints (JWT Required)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/me` | Get current user profile |
| POST | `/simulate` | Start data simulation |
| POST | `/simulate/stop` | Stop running simulation |
| GET | `/simulate/status` | Get simulation status and metrics |
| POST | `/simulate/reset` | Reset all generated data |

Full API documentation is available in [API.md](./API.md).

## Architecture

### Core Components

```
vital-fold-engine/
├── src/
│   ├── main.rs              # Application entry point
│   ├── routes.rs            # Route configuration
│   ├── db.rs                # Database connection setup
│   ├── errors.rs            # Error types and handling
│   ├── config.rs            # Configuration management
│   ├── engine_state.rs       # Simulation state management
│   │
│   ├── handlers/            # Request handlers
│   │   ├── health.rs
│   │   ├── auth.rs
│   │   ├── user.rs
│   │   └── simulation.rs
│   │
│   ├── middleware/          # Request middleware
│   │   └── auth.rs          # JWT validation
│   │
│   ├── generators/          # Data generation logic
│   │   ├── insurance.rs
│   │   ├── clinic.rs
│   │   ├── provider.rs
│   │   ├── patient.rs
│   │   ├── appointment.rs
│   │   └── medical_record.rs
│   │
│   ├── models/              # Data types
│   │   ├── user.rs
│   │   └── ... (other models)
│   │
│   └── db/                  # Database operations
│       └── ... (query builders)
│
├── migrations/              # Database migrations
├── Cargo.toml              # Dependencies
└── .env                    # Configuration (local)
```

### Technology Stack

| Layer | Technology | Purpose |
|-------|-----------|---------|
| **Web Framework** | Actix-web 4.x | High-performance async HTTP server |
| **Runtime** | Tokio | Async task execution |
| **Database** | Aurora DSQL / PostgreSQL | Serverless data persistence |
| **ORM/Query** | SQLx | Type-safe SQL queries |
| **Authentication** | jsonwebtoken + bcrypt | JWT token and password security |
| **Serialization** | Serde + serde_json | JSON request/response handling |
| **API Docs** | Utoipa + Swagger UI | OpenAPI generation and exploration |
| **Logging** | Tracing + tracing-subscriber | Structured request logging |
| **AWS** | AWS SDK (DSQL, DynamoDB, RDS) | Cloud service integration |

## Configuration

### Environment Variables

Copy `.env.example` to `.env` and configure:

```env
# Server
HOST=127.0.0.1
PORT=8787

# Database (Aurora DSQL)
DSQL_ENDPOINT=your-cluster.dsql.region.on.aws
DSQL_CLUSTER_ENDPOINT=your-cluster.dsql.region.on.aws
DSQL_REGION=us-east-2
DSQL_DB_NAME=postgres
DSQL_USER=admin
DSQL_PORT=5432

# AWS Credentials
AWS_REGION=us-east-2
AWS_ACCESS_KEY_ID=AKIA...
AWS_SECRET_ACCESS_KEY=...

# Database Pool
DB_POOL_SIZE=10

# JWT
JWT_SECRET=your-secret-key-must-be-at-least-32-characters-long
JWT_EXPIRY_HOURS=24

# Logging
RUST_LOG=vital_fold_engine=info,actix_web=info
```

For production deployments, see [INSTALLATION.md](./INSTALLATION.md).

## Development

### Local Development Setup

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install dependencies
cargo build

# Run tests
cargo test

# Run linter
cargo clippy

# Run with logging
RUST_LOG=debug cargo run
```

### Database Development

```bash
# Run migrations
cargo sqlx migrate run

# Revert last migration
cargo sqlx migrate revert

# Create new migration
cargo sqlx migrate add -r <migration_name>
```

For detailed development guide, see [DEVELOPMENT.md](./DEVELOPMENT.md).

## Production Deployment

### Render.com Deployment

1. Push code to GitHub repository
2. Create new Render service connected to GitHub
3. Set build command: `cargo build --release`
4. Set start command: `./target/release/vital_fold_engine`
5. Configure environment variables in Render dashboard
6. Bind Aurora DSQL instance

### Environment-Specific Configuration

**Development**
```env
RUST_LOG=debug
DB_POOL_SIZE=5
```

**Production**
```env
RUST_LOG=info,actix_web=warn
DB_POOL_SIZE=20
JWT_SECRET=<strong-random-string-32chars>
```

For complete deployment guide, see [INSTALLATION.md](./INSTALLATION.md).

## Security Considerations

### Authentication & Authorization

- **JWT Tokens**: Protected endpoints require valid JWT bearer tokens
- **Password Security**: Passwords hashed with bcrypt (cost factor 12)
- **Token Expiry**: Tokens expire after configured duration (default 24 hours)
- **No Session Sharing**: Each request must include valid token

### Best Practices

- ✅ Always use HTTPS in production
- ✅ Rotate JWT_SECRET regularly
- ✅ Use strong passwords for database accounts
- ✅ Enable IAM authentication for Aurora DSQL
- ✅ Restrict database firewall to application IPs
- ✅ Monitor access logs for suspicious activity
- ✅ Never commit `.env` files with real credentials
- ✅ Use secrets management (AWS Secrets Manager recommended)

## Simulation Configuration

### Default Data Generation

The simulation generates synthetic data following a realistic healthcare scenario:

- **Insurance Companies**: 7 fixed major providers
- **Insurance Plans**: Fixed set per company
- **Clinics**: 10 distribution across the US
- **Providers**: 50 (configurable)
- **Patients**: 100 (configurable)
- **Appointments per Patient**: ~3 (configurable)
- **Medical Records per Patient**: ~2 (configurable)

### Customizing Data Volume

Modify simulation configuration in API request or through code:

```rust
// In src/generators/mod.rs
pub struct SimulationConfig {
    pub plans_per_company:        usize,  // default: 3
    pub providers:                usize,  // default: 50
    pub patients:                 usize,  // default: 50_000
    pub appointments_per_patient: usize,  // default: 2
    pub records_per_appointment:  usize,  // default: 1
    pub start_date:               NaiveDate,
    pub end_date:                 NaiveDate,
}
```

## Performance Characteristics

### Benchmarks (on typical hardware)

| Operation | Time | Data Volume |
|-----------|------|-------------|
| Health Check | <1ms | - |
| Start Simulation | 202ms response | 100-1000s records/second |
| Get Status | <5ms | - |
| Full Data Generation | ~30-60s | 100 providers, 100 patients, 300 appointments, 200 records |

### Scalability

- **Concurrent Users**: Handles 100+ simultaneous API requests
- **Data Volume**: Supports millions of synthetic records in database
- **Simulation Speed**: ~1000+ inserts/second on standard PostgreSQL
- **Connection Pooling**: Configurable pool size (default 10, recommended 20 for production)

## Troubleshooting

### Server Won't Start

**Error**: `Address already in use`
```bash
# Kill process on port 8787
lsof -ti:8787 | xargs kill -9
cargo run
```

**Error**: `Cannot connect to database`
```bash
# Check database credentials in .env
# Verify database is running and accessible
psql -h $DSQL_ENDPOINT -U $DSQL_USER -d $DSQL_DB_NAME
```

### API Errors

**401 Unauthorized**: Invalid or missing JWT token
- Login first to get token
- Include `Authorization: Bearer <token>` header

**404 Not Found**: Check endpoint path and HTTP method
- Verify against [API.md](./API.md)
- Use Swagger UI for interactive testing

**500 Internal Server Error**: Check server logs
```bash
RUST_LOG=debug cargo run
```

See [DEVELOPMENT.md](./DEVELOPMENT.md) for more troubleshooting.

## Contributing

### Code Quality

- Run formatter: `cargo fmt`
- Check linting: `cargo clippy`
- Run tests: `cargo test`
- All checks pass before committing

### Reporting Issues

Issues should include:
- Clear description of problem
- Steps to reproduce
- Expected vs actual behavior
- Relevant log output

## License

This project is licensed under the MIT License - see [LICENSE](LICENSE) file for details.

## Support & Documentation

- **API Documentation**: [API.md](./API.md)
- **Installation Guide**: [INSTALLATION.md](./INSTALLATION.md)
- **Development Guide**: [DEVELOPMENT.md](./DEVELOPMENT.md)
- **Architecture Guide**: [ARCHITECTURE.md](./ARCHITECTURE.md)
- **Swagger UI**: http://127.0.0.1:8787/swagger-ui/ (when running)

## Roadmap

### Current Release (v0.1.0)
- ✅ Core data generation engine
- ✅ REST API with authentication
- ✅ Simulation lifecycle control
- ✅ Interactive API documentation

### Future Enhancements
- Real-time WebSocket updates for simulation progress
- Advanced filtering and export options
- Multi-tenant support for different scenarios
- Integration with data warehousing solutions
- CLI tool for local data generation
- Docker containerization and Kubernetes deployment

## Contact & Contributing

For questions, suggestions, or contributions, please open an issue or contact the development team.

---

**VitalFold Engine** — Building tomorrow's healthcare analytics with synthetic data today.

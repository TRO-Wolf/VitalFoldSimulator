# Architecture Guide

Comprehensive technical documentation of VitalFold Engine's system design and implementation.

## Table of Contents

1. [System Overview](#system-overview)
2. [Technology Stack](#technology-stack)
3. [Request Flow](#request-flow)
4. [Module Architecture](#module-architecture)
5. [Data Generation Pipeline](#data-generation-pipeline)
6. [Database Schema](#database-schema)
7. [Authentication & Security](#authentication--security)
8. [Error Handling](#error-handling)
9. [State Management](#state-management)
10. [Performance Optimization](#performance-optimization)
11. [Deployment Architecture](#deployment-architecture)

---

## System Overview

### High-Level Architecture

```
┌─────────────┐
│   Client    │ (Browser, API Client, curl, Postman)
└──────┬──────┘
       │ HTTP/REST
       ▼
┌──────────────────────────────────────────────────────┐
│         Actix-web HTTP Server (Port 8787)           │
│ ┌──────────────────────────────────────────────────┐ │
│ │  Request Processing Pipeline                     │ │
│ │  ├─ Route Matching                              │ │
│ │  ├─ Authentication Middleware (JWT)             │ │
│ │  ├─ Request Handler                             │ │
│ │  └─ Response Serialization                       │ │
│ └──────────────────────────────────────────────────┘ │
└──────┬───────────────────────────────────────────────┘
       │ SQL Queries
       ▼
┌──────────────────────────────────────────────────────┐
│        PostgreSQL / Aurora DSQL Database            │
│ ├─ public.users (Authentication)                   │
│ └─ vital_fold.* (Healthcare Data)                  │
└──────────────────────────────────────────────────────┘
       ▲
       │ Async Data Generation
       │
┌──────┴───────────────────────────────────────────────┐
│    Tokio Runtime (Async Task Execution)             │
│ ├─ Simulation Generator Task                        │
│ ├─ Connection Pool Management                       │
│ └─ Request Handler Concurrency                      │
└────────────────────────────────────────────────────┘
```

### Key Components

| Component | Purpose | Technology |
|-----------|---------|------------|
| **HTTP Server** | Handle incoming REST requests | Actix-web 4.x |
| **Request Router** | Route requests to handlers | Actix-web routing |
| **Authentication** | Validate JWT tokens | jsonwebtoken |
| **Handlers** | Process requests, return responses | Actix-web handlers |
| **Data Generators** | Create synthetic healthcare data | Custom Rust generators |
| **Database** | Persistent data storage | PostgreSQL/Aurora DSQL |
| **Connection Pool** | Manage DB connections | SQLx with Tokio pool |
| **State Management** | Track simulation status | Arc<RwLock<SimulatorState>> |
| **Async Runtime** | Execute concurrent tasks | Tokio |

---

## Technology Stack

### Web Framework

**Actix-web 4.x**
- High-performance async HTTP server
- Built on Tokio runtime
- Minimal overhead, excellent performance
- Comprehensive middleware support

### Database

**Aurora DSQL / PostgreSQL 14+**
- Serverless relational database (DSQL)
- ACID compliance for data integrity
- Complex query support for analytics
- Connection pooling via SQLx

### Authentication

**jsonwebtoken + bcrypt**
- JWT (JSON Web Tokens) for stateless authentication
- HMAC SHA-256 signature verification
- bcrypt for password hashing (cost factor 12)
- Bearer token in HTTP Authorization header

### Data Serialization

**Serde + serde_json**
- Automatic serialization/deserialization
- Type-safe JSON handling
- Minimal runtime overhead
- Support for custom serializers

### API Documentation

**Utoipa + Swagger UI**
- Auto-generate OpenAPI 3.0 spec
- Interactive API documentation
- Request/response schema validation
- Automatic security scheme generation

### Async Runtime

**Tokio**
- Multi-threaded async runtime
- Work-stealing scheduler
- Built-in timers and utilities
- Used by Actix-web and SQLx

### Logging & Observability

**tracing + tracing-actix-web + tracing-subscriber**
- Structured logging framework
- Request/response middleware logging
- Environment-based filter configuration
- Async-compatible logging

### Error Handling

**thiserror**
- Type-safe error definitions
- Automatic Display/Error trait implementation
- Error context preservation
- Type conversions via From trait

---

## Request Flow

### Typical Request Lifecycle

```
1. CLIENT REQUEST
   │
   ├─ HTTP Method + Path + Headers + Body
   │
2. ACTIX-WEB RECEIVES REQUEST
   │
   ├─ Route Matching
   │  ├─ Parse path parameters
   │  ├─ Match against registered routes
   │  └─ Determine handler function
   │
3. MIDDLEWARE EXECUTION
   │
   ├─ Logging Middleware (TracingLogger)
   │  └─ Log request details
   │
   ├─ Authentication Middleware (JWT)
   │  ├─ Extract Authorization header
   │  ├─ Validate JWT signature
   │  ├─ Verify token expiration
   │  └─ Extract claims (user_id, email)
   │
4. HANDLER EXECUTION
   │
   ├─ Extract request data
   │  ├─ JSON body deserialization
   │  ├─ Query parameter parsing
   │  └─ Path parameter conversion
   │
   ├─ Process business logic
   │  ├─ Database queries
   │  ├─ Data transformations
   │  └─ Error handling
   │
5. DATABASE OPERATIONS
   │
   ├─ Connection Pool Checkout
   │  └─ Get available connection
   │
   ├─ SQL Execution
   │  ├─ Type-safe query execution (SQLx)
   │  ├─ Parameter binding
   │  └─ Result fetching
   │
   ├─ Connection Return
   │  └─ Release connection back to pool
   │
6. RESPONSE GENERATION
   │
   ├─ Serialize response data
   │  ├─ Convert Rust types to JSON
   │  └─ Add status code
   │
   ├─ Set response headers
   │  ├─ Content-Type: application/json
   │  └─ Custom headers (if needed)
   │
7. SEND RESPONSE TO CLIENT
   │
   └─ HTTP Status Code + Headers + Body
```

### Example Request: Get User Profile

```
Request:
  GET /api/v1/me HTTP/1.1
  Authorization: Bearer eyJhbGciOiJIUzI1NiIs...
  Content-Type: application/json

Processing:
  1. Route matches: GET /api/v1/me → user::me handler
  2. Auth middleware validates JWT → extracts user_id
  3. Handler executes:
     - Query database: SELECT * FROM users WHERE id = ?
     - Deserialize result to UserProfile struct
  4. Database returns: {id, email, created_at}
  5. Handler constructs response: {user_id, email, created_at}
  6. Response serialized to JSON

Response:
  200 OK
  Content-Type: application/json

  {
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "email": "user@example.com",
    "created_at": "2024-02-15T10:30:00Z"
  }
```

---

## Module Architecture

### Module Organization

```
vital_fold_engine/
│
├── main.rs
│   └─ Application entry point
│      ├─ Initialize logging
│      ├─ Load configuration
│      ├─ Create database pool
│      ├─ Setup OpenAPI docs
│      └─ Start HTTP server
│
├── routes.rs
│   └─ Route configuration
│      ├─ Public routes (health, auth)
│      └─ Protected routes (simulation, user)
│
├── config.rs
│   └─ Configuration management
│      ├─ Environment variable loading
│      ├─ Configuration validation
│      └─ Default values
│
├── db.rs
│   └─ Database setup
│      ├─ Connection pool creation
│      ├─ Pool configuration
│      └─ Health check
│
├── errors.rs
│   └─ Error types
│      ├─ AppError enum
│      ├─ Error conversions
│      └─ HTTP response mapping
│
├── engine_state.rs
│   └─ Global simulation state
│      ├─ SimulatorState struct
│      ├─ RwLock for thread-safe access
│      └─ State update methods
│
├── handlers/
│   ├── mod.rs
│   ├── health.rs
│   │   └─ GET /health
│   ├── auth.rs
│   │   ├─ POST /api/v1/auth/register
│   │   └─ POST /api/v1/auth/login
│   ├── user.rs
│   │   └─ GET /api/v1/me
│   └── simulation.rs
│       ├─ POST /simulate (start)
│       ├─ POST /simulate/stop
│       ├─ GET /simulate/status
│       └─ POST /simulate/reset
│
├── middleware/
│   ├── mod.rs
│   └── auth.rs
│       └─ JWT validation middleware
│
├── models/
│   ├── mod.rs
│   └── user.rs
│       ├─ RegisterRequest
│       ├─ LoginRequest
│       ├─ AuthResponse
│       ├─ UserProfile
│       └─ Other DTOs
│
├── generators/
│   ├── mod.rs
│   │   ├─ SimulationConfig
│   │   ├─ SimulationContext
│   │   └─ run_simulation orchestration
│   ├── insurance.rs
│   │   ├─ generate_insurance_companies
│   │   └─ generate_insurance_plans
│   ├── clinic.rs
│   │   ├─ generate_clinics
│   │   └─ generate_clinic_schedules
│   ├── provider.rs
│   │   └─ generate_providers
│   ├── patient.rs
│   │   ├─ generate_patients
│   │   ├─ generate_emergency_contacts
│   │   ├─ generate_patient_demographics
│   │   └─ generate_patient_insurance
│   ├── appointment.rs
│   │   └─ generate_appointments
│   └── medical_record.rs
│       └─ generate_medical_records
│
└── db/ (if needed)
    └─ Query builders and helpers
```

### Inter-Module Dependencies

```
main.rs
  ├─ routes.rs
  ├─ handlers/* (all)
  ├─ middleware/auth.rs
  ├─ config.rs
  ├─ db.rs
  └─ engine_state.rs

routes.rs
  ├─ handlers/* (references)
  └─ middleware/auth.rs

handlers/simulation.rs
  ├─ generators/mod.rs (run_simulation)
  ├─ engine_state.rs (SimulatorState)
  └─ db.rs (DbPool)

generators/mod.rs
  ├─ generators/insurance.rs
  ├─ generators/clinic.rs
  ├─ generators/provider.rs
  ├─ generators/patient.rs
  ├─ generators/appointment.rs
  └─ generators/medical_record.rs

middleware/auth.rs
  └─ models/user.rs (Claims)
```

---

## Data Generation Pipeline

### Simulation Orchestration

The `run_simulation` function coordinates the entire data generation process:

```rust
pub async fn run_simulation(
    pool: DbPool,
    config: SimulationConfig,
    state: &SimulatorState,
) -> Result<(), AppError> {
    // 1. Create simulation context
    let mut ctx = SimulationContext::new(pool, config);

    // 2. Execute generation steps in dependency order
    insurance::generate_insurance_companies(&mut ctx).await?;
    insurance::generate_insurance_plans(&mut ctx).await?;
    clinic::generate_clinics(&mut ctx).await?;
    provider::generate_providers(&mut ctx).await?;
    patient::generate_patients(&mut ctx).await?;
    patient::generate_emergency_contacts(&mut ctx).await?;
    patient::generate_patient_demographics(&mut ctx).await?;
    patient::generate_patient_insurance(&mut ctx).await?;
    clinic::generate_clinic_schedules(&mut ctx).await?;
    appointment::generate_appointments(&mut ctx).await?;
    medical_record::generate_medical_records(&mut ctx).await?;

    // 3. Update global state with final counts
    state.set_last_run(Utc::now());
    state.set_counts(ctx.counts);

    Ok(())
}
```

### Generation Order & Dependencies

```
Step 1: Insurance Companies (Fixed: 7)
        ↓ (must exist before plans)
Step 2: Insurance Plans (Fixed)
        ↓ (must exist before patient insurance)

Step 3: Clinics (Fixed: 10 distribution)
        ├─ Step 9: Clinic Schedules
        │           ↓ (must exist before appointments)
        └─ No data dependency

Step 4: Providers (N configurable)
        ↓ (must exist before appointments)

Step 5: Patients (N configurable)
        ├─ Step 6: Emergency Contacts (1 per patient)
        ├─ Step 7: Patient Demographics (1 per patient)
        └─ Step 8: Patient Insurance Links
                   ↓ (must exist before appointments)

Step 10: Appointments (M configurable per patient)
         ├─ Requires: Clinics, Providers, Patients
         ├─ Requires: Clinic Schedules
         ├─ Requires: Patient Insurance
         └─ Step 11: Medical Records (per appointment)
                     ↓ (must exist after appointments)
```

### SimulationContext

The `SimulationContext` struct coordinates data generation:

```rust
pub struct SimulationContext {
    // Database connection
    pub pool: DbPool,

    // Configuration parameters
    pub config: SimulationConfig,

    // Accumulated entity counts
    pub counts: SimulationCounts,

    // IDs of generated entities (for references)
    pub insurance_company_ids: Vec<Uuid>,
    pub insurance_plan_ids: Vec<Uuid>,
    pub clinic_ids: Vec<Uuid>,
    pub provider_ids: Vec<Uuid>,
    pub patient_ids: Vec<Uuid>,
    pub clinic_schedule_ids: Vec<Uuid>,
}
```

**Purpose:**
- Pass database connections through generator chain
- Store generated IDs for referential integrity
- Track entity counts for final report
- Share configuration across all generators

### Generator Pattern

Each generator module follows a consistent pattern:

```rust
pub async fn generate_something(ctx: &mut SimulationContext) -> Result<(), AppError> {
    // 1. GENERATE: Create data structures
    let items = vec![
        // Synthesized data using Faker library
    ];

    // 2. INSERT: Batch insert into database
    for item in &items {
        sqlx::query!(
            "INSERT INTO something (...) VALUES (...)",
            // parameters
        )
        .execute(&ctx.pool)
        .await?;
    }

    // 3. TRACK: Store IDs and update counts
    ctx.something_ids = items.iter().map(|i| i.id).collect();
    ctx.counts.something = items.len();

    Ok(())
}
```

---

## Database Schema

### Authentication Schema (public.users)

```sql
CREATE TABLE public.users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_email ON public.users(email);
```

### Healthcare Schema (vital_fold.*)

Generated by `health_clinic_schema.sql` at setup:

```
vital_fold.insurance_companies
  ├─ id (UUID)
  ├─ name (VARCHAR)
  ├─ country (VARCHAR)
  └─ region (VARCHAR)

vital_fold.insurance_plans
  ├─ id (UUID)
  ├─ insurance_company_id (FK)
  ├─ plan_name (VARCHAR)
  └─ plan_type (VARCHAR)

vital_fold.clinics
  ├─ id (UUID)
  ├─ clinic_name (VARCHAR)
  ├─ address (VARCHAR)
  ├─ city (VARCHAR)
  ├─ state (VARCHAR)
  ├─ zip_code (VARCHAR)
  └─ phone_number (VARCHAR)

vital_fold.providers
  ├─ id (UUID)
  ├─ first_name (VARCHAR)
  ├─ last_name (VARCHAR)
  ├─ specialty (VARCHAR)
  ├─ license_number (VARCHAR)
  └─ clinic_id (FK)

vital_fold.patients
  ├─ id (UUID)
  ├─ first_name (VARCHAR)
  ├─ last_name (VARCHAR)
  ├─ date_of_birth (DATE)
  ├─ gender (VARCHAR)
  ├─ marital_status (VARCHAR)
  ├─ phone_number (VARCHAR)
  └─ email (VARCHAR)

vital_fold.emergency_contacts
  ├─ id (UUID)
  ├─ patient_id (FK)
  ├─ name (VARCHAR)
  ├─ relationship (VARCHAR)
  └─ phone_number (VARCHAR)

vital_fold.patient_demographics
  ├─ id (UUID)
  ├─ patient_id (FK)
  ├─ ethnicity (VARCHAR)
  ├─ language (VARCHAR)
  └─ occupation (VARCHAR)

vital_fold.patient_insurance
  ├─ id (UUID)
  ├─ patient_id (FK)
  ├─ insurance_plan_id (FK)
  ├─ member_id (VARCHAR)
  └─ coverage_start_date (DATE)

vital_fold.clinic_schedules
  ├─ id (UUID)
  ├─ clinic_id (FK)
  ├─ day_of_week (VARCHAR)
  ├─ open_time (TIME)
  └─ close_time (TIME)

vital_fold.appointments
  ├─ id (UUID)
  ├─ patient_id (FK)
  ├─ provider_id (FK)
  ├─ clinic_id (FK)
  ├─ appointment_date (TIMESTAMPTZ)
  ├─ appointment_type (VARCHAR)
  └─ duration_minutes (INTEGER)

vital_fold.medical_records
  ├─ id (UUID)
  ├─ appointment_id (FK)
  ├─ patient_id (FK)
  ├─ provider_id (FK)
  ├─ diagnosis (VARCHAR)
  ├─ notes (TEXT)
  └─ created_at (TIMESTAMPTZ)
```

### Referential Integrity

```
insurance_plans → insurance_companies
patient_insurance → insurance_plans
patient_insurance → patients
clinic_schedules → clinics
providers → clinics
appointments → patients
appointments → providers
appointments → clinics
appointments → clinic_schedules
medical_records → appointments
medical_records → patients
medical_records → providers
emergency_contacts → patients
patient_demographics → patients
```

---

## Authentication & Security

### JWT Token Structure

```
Header.Payload.Signature

Header (Base64):
{
  "alg": "HS256",
  "typ": "JWT"
}

Payload (Base64):
{
  "sub": "550e8400-e29b-41d4-a716-446655440000",
  "email": "user@example.com",
  "iat": 1771808822,
  "exp": 1771895222
}

Signature (HMAC-SHA256):
HMACSHA256(
  base64UrlEncode(header) + "." + base64UrlEncode(payload),
  JWT_SECRET
)
```

### Authentication Flow

```
1. User Registers
   POST /api/v1/auth/register
   {email, password} → Hash password with bcrypt → Store in DB → Generate JWT

2. User Logs In
   POST /api/v1/auth/login
   {email, password} → Verify password with bcrypt → Generate JWT → Return to client

3. Client Stores Token
   Save token in secure storage (localStorage, cookie, etc.)

4. Client Makes Request
   GET /api/v1/me
   Authorization: Bearer <token>

5. Server Validates Token
   ├─ Extract token from Authorization header
   ├─ Verify signature with JWT_SECRET
   ├─ Check token expiration
   ├─ Extract claims (user_id, email)
   └─ Add claims to request context

6. Handler Accesses Claims
   let claims = req.extensions().get::<Claims>();
   let user_id = claims.sub;
```

### Middleware Implementation

```rust
pub async fn jwt_validator(
    req: ServiceRequest,
    _: Rc<JwtValidator>,
) -> Result<ServiceRequest, actix_web::Error> {
    // Extract Authorization header
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok());

    // Verify "Bearer " prefix
    let token = auth_header
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| UnauthorizedError::MissingToken)?;

    // Decode and validate JWT
    let claims = jsonwebtoken::decode::<Claims>(
        &token,
        &DecodingKey::from_secret(JWT_SECRET.as_ref()),
        &Validation::default(),
    )
    .map_err(|_| UnauthorizedError::InvalidToken)?
    .claims;

    // Add claims to request extensions
    req.extensions_mut().insert(claims);

    Ok(req)
}
```

### Password Security

```rust
// Registration: Hash password
let password_hash = bcrypt::hash(&password, 12)?;
// Store password_hash in database

// Login: Verify password
let is_valid = bcrypt::verify(&provided_password, &stored_hash)?;
```

**bcrypt Configuration:**
- Cost factor: 12 (industry standard)
- Algorithm: 2b (current version)
- ~100ms per hash operation (intentional slowdown for security)

### Security Best Practices

✅ **Implemented:**
- JWT bearer token authentication
- Password hashing with bcrypt
- Token signature verification
- Token expiration checking
- HTTPS-ready (via reverse proxy)
- SQL injection prevention (prepared statements via SQLx)
- Secure random UUID generation

⚠️ **Not Implemented (Production Considerations):**
- Token refresh mechanism
- Revocation list (blacklist expired tokens)
- Rate limiting
- CORS policy
- Input validation middleware
- API key authentication
- OAuth2 integration
- MFA support

---

## Error Handling

### AppError Enum

```rust
pub enum AppError {
    NotFound(String),
    BadRequest(String),
    Unauthorized(String),
    Conflict(String),
    InternalError(String),
    DatabaseError(sqlx::Error),
}
```

### Error Responses

```rust
// All errors converted to HTTP responses
impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        match self {
            AppError::NotFound(msg) => {
                HttpResponse::NotFound().json(ErrorResponse {
                    error: "NOT_FOUND".to_string(),
                    message: msg.clone(),
                })
            },
            AppError::Unauthorized(msg) => {
                HttpResponse::Unauthorized().json(ErrorResponse {
                    error: "UNAUTHORIZED".to_string(),
                    message: msg.clone(),
                })
            },
            // ... other variants
        }
    }
}
```

### Error Flow

```
Handler Code
    ↓
Error occurs (Result::Err)
    ↓
AppError variant created
    ↓
? operator propagates error
    ↓
error_response() converts to HTTP response
    ↓
Response sent to client
```

---

## State Management

### Global Simulation State

```rust
pub struct SimulatorState {
    running: Arc<RwLock<bool>>,
    last_run: Arc<RwLock<Option<DateTime<Utc>>>>,
    counts: Arc<RwLock<SimulationCounts>>,
}
```

**Thread-Safe Access:**
- `Arc<T>` - Atomic Reference Count for sharing across threads
- `RwLock<T>` - Read-Write Lock for safe concurrent access
- Multiple readers allowed
- Exclusive access for writers

### State Update Pattern

```rust
// Update running status
state.try_start(); // Atomic transition: false → true

// Update counts after simulation
state.set_counts(new_counts);

// Read state
let is_running = state.is_running(); // Acquires read lock
```

### Concurrency Guarantees

```
Client 1: Start Simulation
   ├─ Acquires write lock on running
   ├─ Sets running = true
   ├─ Spawns async task
   └─ Releases lock

Client 2: Check Status (concurrent)
   ├─ Acquires read lock on running
   ├─ Reads running = true
   ├─ Reads counts = {...}
   └─ Releases lock

Background: Simulation Task
   ├─ Inserts data into database
   ├─ Updates context counts
   ├─ Acquires write lock on counts
   ├─ Updates counts
   └─ Releases lock
```

---

## Performance Optimization

### Connection Pooling

```rust
// SQLx creates pool of N connections
let pool = sqlx::postgres::PgPoolOptions::new()
    .max_connections(config.db_pool_size)
    .connect(&database_url)
    .await?;

// Connection checkout/return is O(1)
// No connection creation overhead per request
```

**Configuration:**
- Default pool size: 10 connections
- Production recommended: 15-20 connections
- Tuned based on concurrent workload

### Async Request Handling

```rust
// Actix-web spawns new task per request
// Non-blocking I/O via Tokio
pub async fn handler(pool: web::Data<DbPool>) -> Result<...> {
    // Database query suspends thread (doesn't block)
    let user = sqlx::query_as::<_, User>(...)
        .fetch_one(&pool)
        .await?;  // ← Async await, not blocking

    // Other requests continue executing
}
```

**Benefits:**
- Handle 100+ concurrent requests on single server
- Low thread count (number of CPU cores)
- Efficient CPU utilization

### Query Optimization

```rust
// Prepared statements (via SQLx)
sqlx::query_as::<_, User>(
    "SELECT id, email FROM users WHERE id = ?"
)
.bind(user_id)
.fetch_one(&pool)
.await?;

// Type-safe queries prevent SQL injection
// Query compilation at compile-time
// Database uses query plan caching
```

### Data Generation Performance

```rust
// Batch inserts (faster than individual inserts)
let mut tx = pool.begin().await?;
for item in &items {
    sqlx::query!("INSERT INTO table VALUES (...)").execute(&mut *tx).await?;
}
tx.commit().await?;

// Performance: ~1000+ inserts/second on standard PostgreSQL
```

---

## Deployment Architecture

### Local Development

```
Developer Machine
    ├─ Rust compiler (cargo build)
    ├─ PostgreSQL server (localhost:5432)
    ├─ Actix-web HTTP server (:8787)
    └─ IDE (VS Code, IntelliJ, etc.)
```

### Render.com Deployment

```
GitHub Repository
    ↓ (push code)
Render.com CI/CD Pipeline
    ├─ Clone repository
    ├─ Run: cargo build --release
    ├─ Deploy binary
    └─ Start process
Render.com Infrastructure
    ├─ Web Service Container (:3000)
    ├─ Environment variables
    └─ Auto-scaling (if enabled)
PostgreSQL Database
    └─ Managed database instance
```

### Aurora DSQL Deployment

```
AWS Account
    ├─ Aurora DSQL Cluster
    │   ├─ Serverless v2
    │   ├─ Auto-scaling capacity
    │   └─ Automated backups
    │
    ├─ Application Server (EC2, ECS, or Render)
    │   ├─ Vital-Fold-Engine binary
    │   └─ Connection pooling
    │
    └─ IAM Roles & Security Groups
        ├─ Database access credentials
        └─ Network security
```

### Docker Containerization

```
Dockerfile
    ├─ Multi-stage build
    ├─ Builder stage: cargo build --release
    ├─ Runtime stage: Debian slim
    └─ Binary: ./vital-fold-engine

Docker Image
    ├─ Tag: vital-fold-engine:latest
    ├─ Size: ~50-100 MB (release binary)
    └─ Base: debian:bookworm-slim

Container Runtime
    ├─ Port 8787 exposed
    ├─ Environment variables injected
    └─ Health checks enabled
```

### Production Considerations

**Security:**
- ✅ HTTPS/TLS (reverse proxy)
- ✅ Database firewall rules
- ✅ AWS IAM authentication
- ✅ Secrets in AWS Secrets Manager

**Reliability:**
- ✅ Connection pooling
- ✅ Health check endpoint
- ✅ Structured logging
- ✅ Error tracking
- ✅ Database backups
- ✅ Load balancing (optional)

**Monitoring:**
- ⚠️ CloudWatch logs (needs setup)
- ⚠️ Application metrics (not yet implemented)
- ⚠️ Database monitoring (AWS native)
- ⚠️ Alert thresholds (not yet configured)

---

## Scaling Considerations

### Vertical Scaling (Single Server)

**Increase capacity:**
- Larger server (more RAM, CPU)
- Larger database pool (more connections)
- Optimize query performance

**Limits:**
- Single point of failure
- Cost increases with server size
- Database becomes bottleneck

### Horizontal Scaling (Multiple Servers)

**Required changes:**
- Load balancer (distribute requests)
- Shared database (already stateless)
- Distributed session management (not needed - JWT)

**Benefits:**
- Handle more concurrent users
- Fault tolerance
- Cost-effective scaling

### Database Scaling

**Aurora DSQL advantages:**
- Serverless auto-scaling
- Managed backups
- Multi-AZ replication
- Read replicas available

**Optimization options:**
- Connection pooling (PgBouncer)
- Caching layer (Redis)
- Read replicas for analytics

---

## Future Architecture Improvements

1. **Caching Layer**
   - Redis for session caching
   - Query result caching
   - Reduced database load

2. **Message Queue**
   - Async task processing
   - Long-running simulations
   - Job persistence

3. **Monitoring & Observability**
   - Prometheus metrics
   - Grafana dashboards
   - Distributed tracing

4. **API Gateway**
   - Rate limiting
   - Request routing
   - Centralized authentication

5. **Microservices**
   - Separate data generation service
   - Independent scaling
   - Technology flexibility


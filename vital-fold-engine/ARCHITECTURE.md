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
12. [Scaling Considerations](#scaling-considerations)
13. [Three-Phase Data Lifecycle](#three-phase-data-lifecycle)
14. [Frontend Dashboard](#frontend-dashboard)
15. [Progress Tracking](#progress-tracking)
16. [Visualization Pipeline](#visualization-pipeline)

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
│   │   ├─ POST /api/v1/auth/login
│   │   └─ POST /api/v1/auth/admin-login
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
    pub clinic_ids: Vec<i64>,                // BIGINT identity columns
    pub provider_ids: Vec<i64>,              // BIGINT identity columns
    pub patient_ids: Vec<Uuid>,
    pub patient_home_clinics: Vec<usize>,    // Patient → clinic index (for geographic bias)
    pub provider_clinic_assignments: Vec<usize>, // Provider → clinic index (proportional distribution)
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
vital_fold.insurance_company
  ├─ company_id (UUID)
  ├─ company_name (VARCHAR)
  ├─ email (VARCHAR)
  ├─ phone_number (VARCHAR)
  └─ tax_id_number (INT)

vital_fold.insurance_plan
  ├─ insurance_plan_id (UUID)
  ├─ plan_name (VARCHAR)
  ├─ company_id (FK → insurance_company)
  ├─ deductible_amount (DECIMAL)
  ├─ copay_amount (DECIMAL)
  ├─ prior_auth_required (BOOLEAN)
  ├─ active_plan (BOOLEAN)
  └─ active_start_date (DATE)

vital_fold.provider
  ├─ provider_id (BIGINT IDENTITY CACHE 1)    -- NOT UUID
  ├─ first_name (VARCHAR)
  ├─ last_name (VARCHAR)
  ├─ specialty (VARCHAR)                       -- Cardiologist, Cardiac Surgeon, etc.
  ├─ license_type (VARCHAR)                    -- "MD" | "DO" | "NP" (~30% NPs)
  ├─ phone_number (VARCHAR)
  └─ email (VARCHAR)                            -- "j.smith@example.org"

vital_fold.clinic
  ├─ clinic_id (BIGINT IDENTITY CACHE 1)       -- NOT UUID
  ├─ clinic_name (VARCHAR)                     -- "VitalFold Heart Center - Miami 1"
  ├─ region (VARCHAR)
  ├─ street_address (VARCHAR)                  -- "1234 Elm Blvd, Suite 200"
  ├─ city (VARCHAR)
  ├─ state (VARCHAR)
  ├─ zip_code (VARCHAR)                        -- Metro-prefix + 2 digits
  ├─ phone_number (VARCHAR)
  └─ email (VARCHAR)                            -- "vfhc_miami1@vitalfold.org"

vital_fold.patient
  ├─ patient_id (UUID)
  ├─ first_name, last_name, middle_name (VARCHAR)
  ├─ date_of_birth (DATE)
  ├─ street_address, city, state, zip_code (VARCHAR)
  ├─ phone_number, email (VARCHAR)
  ├─ registration_date (DATE)
  └─ emergency_contact_id (VARCHAR)

vital_fold.emergency_contact
  ├─ emergency_contact_id (UUID)
  ├─ patient_id (FK → patient)
  ├─ first_name, last_name (VARCHAR)
  ├─ relationship (VARCHAR)
  ├─ phone_number, email (VARCHAR)

vital_fold.patient_demographics
  ├─ demographics_id (UUID)
  ├─ patient_id (FK → patient)
  ├─ first_name, last_name, date_of_birth (duplicated)
  ├─ age (INT)
  ├─ ssn (VARCHAR)
  ├─ ethnicity (VARCHAR)
  └─ birth_gender (VARCHAR)

vital_fold.patient_insurance
  ├─ patient_insurance_id (UUID)
  ├─ patient_id (FK → patient)
  ├─ insurance_plan_id (FK → insurance_plan)
  ├─ policy_number (VARCHAR)
  ├─ coverage_start_date (DATE)                -- Random within past 365 days
  └─ coverage_end_date (DATE NULL)             -- ~20% populated (expired)

vital_fold.clinic_schedule
  ├─ schedule_id (UUID)
  ├─ clinic_id (BIGINT FK → clinic)
  ├─ provider_id (BIGINT FK → provider)
  ├─ day_of_week (VARCHAR)
  ├─ start_time (TIME)                         -- 08:00
  └─ end_time (TIME)                           -- 17:00

vital_fold.appointment
  ├─ appointment_id (UUID)
  ├─ patient_id (FK → patient)
  ├─ provider_id (BIGINT FK → provider)
  ├─ clinic_id (BIGINT FK → clinic)
  ├─ appointment_datetime (TIMESTAMP)          -- 15-minute windows, 8:00–16:45
  └─ reason_for_visit (VARCHAR)

vital_fold.medical_record
  ├─ medical_record_id (UUID)
  ├─ patient_id (FK → patient)
  ├─ provider_id (BIGINT FK → provider)
  ├─ clinic_id (BIGINT FK → clinic)
  ├─ record_date (TIMESTAMP)
  ├─ diagnosis (VARCHAR)                       -- 8 fixed cardiac codes
  └─ treatment (VARCHAR)

vital_fold.patient_visit
  ├─ patient_visit_id (UUID)
  ├─ appointment_id (UUID FK → appointment)    -- explicit link to originating appointment
  ├─ patient_id (FK → patient)
  ├─ clinic_id (BIGINT FK → clinic)
  ├─ provider_id (BIGINT FK → provider)
  ├─ checkin_time (TIMESTAMP)                  -- 5-15 min before appointment
  ├─ checkout_time (TIMESTAMP)                 -- 15-30 min after appointment
  ├─ provider_seen_time (TIMESTAMP)            -- 0-5 min after appointment
  ├─ ekg_usage (BOOLEAN)                       -- ~20% true
  ├─ estimated_copay (DECIMAL)                 -- $150-$350 EKG, $20-$150 standard
  ├─ creation_time (TIMESTAMP)
  └─ record_expiration_epoch (BIGINT)

vital_fold.patient_vitals
  ├─ patient_visit_id (UUID PK + FK → patient_visit)   -- 1:1 relationship
  ├─ patient_id, clinic_id, provider_id (FKs)
  ├─ height, weight (DECIMAL)
  ├─ blood_pressure (VARCHAR)                  -- "120/80"
  ├─ heart_rate (INT)
  ├─ temperature, oxygen_saturation (DECIMAL)
  ├─ creation_time (TIMESTAMP)
  └─ record_expiration_epoch (BIGINT)
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
1. User Logs In
   POST /api/v1/auth/login
   {email, password} → Verify password with bcrypt → Generate JWT → Return to client

2. Client Stores Token
   Save token in secure storage (localStorage, cookie, etc.)

3. Client Makes Request
   GET /api/v1/me
   Authorization: Bearer <token>

4. Server Validates Token
   ├─ Extract token from Authorization header
   ├─ Verify signature with JWT_SECRET
   ├─ Check token expiration
   ├─ Extract claims (user_id, email)
   └─ Add claims to request context

5. Handler Accesses Claims
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

## Three-Phase Data Lifecycle

The system uses a three-phase pipeline separating concerns between reference data, date-dependent data, and DynamoDB replication.

### Phase 1: Static Populate (`POST /populate/static`)

Seeds Aurora DSQL with immutable reference data (8 steps):
1. Insurance companies (7 fixed carriers)
2. Insurance plans (N per company)
3. Clinics (10 fixed SE US locations)
4. Providers (N cardiac specialists)
5. Patients (N with geographically distributed addresses)
6. Emergency contacts (1:1 with patients)
7. Patient demographics (SSN, ethnicity, age)
8. Patient insurance links (random plan assignment)

Run once. Returns `409 Conflict` if data already exists.

### Phase 2: Dynamic Populate (`POST /populate/dynamic`)

Seeds date-dependent data for a configurable date range (5 steps):
1. Clinic schedules (provider-clinic-day combinations) — first run only
2. **Appointments (provider-driven, 36 slots/day per provider)**
3. Medical records (N per appointment with cardiac diagnoses)
4. Patient visits (checkin 5-15 min before, checkout 15-30 min after, linked via `appointment_id`)
5. Patient vitals (height, weight, BP, HR, temp, O2) — 1:1 with patient_visit

**Appointment volume is deterministic**, not configurable:
- Each provider fills exactly 36 slots per day (8:00 AM – 4:45 PM, 15-minute windows)
- Providers are distributed across clinics proportionally via `clinic_weights`
- A clinic with 4 providers generates 144 appointments/day; with 50 providers total, ~1,800 appts/day across all clinics

Can be called multiple times for different date ranges. Validates no overlap with existing dates.

### Phase 3: DynamoDB Sync (`POST /simulate/date-range`)

Reads Aurora `patient_visit` JOIN `patient_vitals` for a date range and writes to two DynamoDB tables. No Aurora data generation. Bounded at 40 concurrent writes per table with exponential backoff on throttling.

### Orchestration

All phases use the fire-and-poll pattern:
- POST returns `202 Accepted` immediately
- Background task runs via `tokio::spawn`
- `AtomicBool` running flag prevents parallel operations
- Poll `GET /simulate/status` for progress

---

## Frontend Dashboard

The engine serves a single-page application at the root URL (`/`) via `actix_files::Files`.

### Technology

- **Framework:** Preact 10 (3KB React alternative) + HTM 3 (tagged template literals)
- **CSS:** Pico CSS (semantic HTML styling via CDN)
- **Bundling:** None — native ES modules, no build step required
- **Routing:** Hash-based (`#/login`, `#/dashboard`, `#/visitors`)

### Architecture

```
static/
├── index.html                  # SPA entry point, CDN imports
├── css/style.css               # Custom overrides
└── js/
    ├── app.js                  # Hash router
    ├── api.js                  # Fetch wrapper with JWT injection
    ├── pages/
    │   ├── login.js            # Login + admin-login forms
    │   ├── dashboard.js        # Main control panel
    │   └── visitors.js         # Per-clinic visitor list
    └── components/
        ├── nav.js                    # Top navigation
        ├── status-badge.js           # Running/idle indicator
        ├── count-table.js            # Aurora + DynamoDB row counts
        ├── populate-form.js          # Static populate config form
        ├── dynamic-populate-form.js  # Dynamic populate date range form
        ├── date-range-form.js        # DynamoDB sync date range form
        ├── populate-calendar.js      # Visual calendar of populated dates
        ├── confirm-modal.js          # Reset confirmation dialogs
        └── heatmap.js                # Per-clinic activity visualization
```

### Auth Flow

1. User enters credentials on login page
2. `api.js` sends POST to `/api/v1/auth/admin-login`
3. JWT stored in `sessionStorage`
4. All subsequent fetch calls include `Authorization: Bearer <token>` header
5. 401 response clears token and redirects to login

---

## Progress Tracking

Three progress structures enable real-time UI updates during long-running operations:

### `PopulateProgress`

Published during any populate operation (static, dynamic, or legacy full).

```rust
pub struct PopulateProgress {
    pub current_step: String,    // e.g., "Appointments"
    pub steps_done: usize,       // 0-based step index
    pub total_steps: usize,      // 8 (static), 5 (dynamic), or 13 (full)
    pub rows_written: u64,       // Cumulative Aurora rows
    pub is_complete: bool,
}
```

### `ResetProgress`

Published during Aurora data reset (`POST /simulate/reset`).

```rust
pub struct ResetProgress {
    pub current_table: String,   // e.g., "patient_vitals"
    pub tables_done: usize,
    pub total_tables: usize,     // 13
    pub rows_deleted: u64,
    pub is_complete: bool,
}
```

### `DynamoProgress`

Published during DynamoDB sync or reset operations.

```rust
pub struct DynamoProgress {
    pub operation: String,       // e.g. "Syncing to DynamoDB"
    pub current_table: String,   // "patient_visit" or "patient_vitals"
    pub tables_done: usize,
    pub total_tables: usize,
    pub items_processed: u64,
    pub total_items: u64,        // 0 if unknown (e.g. scan-delete)
    pub is_complete: bool,
}
```

All three appear as optional fields in the `GET /simulate/status` response, serialized with `#[serde(skip_serializing_if = "Option::is_none")]`.

---

## Visualization Pipeline

### Timelapse (`POST /simulate/timelapse`)

Animates appointment activity across populated dates:

1. Queries Aurora for distinct appointment dates
2. For each day, iterates hours 9 AM through 5 PM
3. For each hour-window, counts appointments per clinic
4. Publishes `TimelapseState` to `SimulatorState` for UI polling
5. Sleeps `window_interval_secs` (default: 5) between updates
6. Auto-populates DynamoDB for the day if not already synced

### Heatmap (`GET /simulate/heatmap`)

Returns the current `TimelapseState`:

```rust
pub struct TimelapseState {
    pub simulation_day: String,       // "2026-04-15"
    pub day_number: usize,            // 1-based
    pub total_days: usize,
    pub sim_hour: u32,                // 9-17
    pub clinics: Vec<ClinicActivity>,
}

pub struct ClinicActivity {
    pub clinic_id: String,
    pub city: String,
    pub state: String,
    pub active_patients: usize,
}
```

### Replay (`POST /simulate/replay`)

Read-only variant of timelapse — queries Aurora directly without writing to DynamoDB. Same visualization, no side effects.

### Visitors (`GET /simulate/visitors`)

Returns today's patient names grouped by clinic with appointment times. Queries Aurora `appointment` JOIN `patient` for the current date.

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


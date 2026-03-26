# API Reference

Complete documentation for all VitalFold Engine API endpoints.

## Table of Contents

1. [Authentication](#authentication)
2. [Public Endpoints](#public-endpoints)
3. [Protected Endpoints](#protected-endpoints)
4. [Response Formats](#response-formats)
5. [Error Handling](#error-handling)
6. [Rate Limiting](#rate-limiting)

---

## Authentication

### JWT Bearer Token

Protected endpoints require authentication using JWT bearer tokens.

**How to Authenticate:**

1. Login to get a JWT token
2. Include token in request header:

```http
Authorization: Bearer <jwt-token>
```

**Example:**

```bash
curl -X GET http://127.0.0.1:8787/api/v1/me \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
```

**Token Structure:**

- **Algorithm**: HMAC SHA-256
- **Format**: Three base64-encoded parts separated by dots
- **Payload**: Contains user ID and email
- **Expiry**: Configurable (default 24 hours)
- **Refresh**: Obtain new token by logging in again

**Token Claims:**

```json
{
  "sub": "550e8400-e29b-41d4-a716-446655440000",
  "email": "user@example.com",
  "iat": 1771808822,
  "exp": 1771895222
}
```

---

## Public Endpoints

### 1. Health Check

Check if the API is running and healthy.

**Endpoint:**
```
GET /health
```

**Authentication:** None

**Request:**
```bash
curl http://127.0.0.1:8787/health
```

**Response:** `200 OK`
```json
{
  "status": "healthy"
}
```

**Use Cases:**
- Load balancer health checks
- Monitoring and uptime verification
- Application startup verification

---

### 2. Login User

Authenticate with existing credentials and receive JWT token.

**Endpoint:**
```
POST /api/v1/auth/login
```

**Authentication:** None

**Request Body:**
```json
{
  "email": "user@example.com",
  "password": "SecurePassword123"
}
```

**Request:**
```bash
curl -X POST http://127.0.0.1:8787/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "SecurePassword123"
  }'
```

**Response:** `200 OK`
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "email": "user@example.com"
}
```

**Status Codes:**
- `200 OK` - Login successful
- `400 Bad Request` - Missing email or password
- `401 Unauthorized` - Invalid credentials
- `500 Internal Server Error` - Database error

**Token Usage:**
- Token valid for configured duration (default 24 hours)
- Include in `Authorization: Bearer` header for protected endpoints
- Login again to refresh token

---

## Protected Endpoints

All protected endpoints require valid JWT token in `Authorization: Bearer` header.

### 1. Get Current User Profile

Retrieve the profile of the authenticated user.

**Endpoint:**
```
GET /api/v1/me
```

**Authentication:** JWT Bearer Token (Required)

**Request Headers:**
```http
Authorization: Bearer <jwt-token>
Content-Type: application/json
```

**Request:**
```bash
TOKEN="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."

curl -X GET http://127.0.0.1:8787/api/v1/me \
  -H "Authorization: Bearer $TOKEN"
```

**Response:** `200 OK`
```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "email": "user@example.com",
  "created_at": "2024-02-15T10:30:00Z"
}
```

**Status Codes:**
- `200 OK` - User profile retrieved
- `401 Unauthorized` - Missing or invalid token
- `404 Not Found` - User not found in database
- `500 Internal Server Error` - Database error

**Use Cases:**
- Verify current user information
- Get user ID for logging/auditing
- Validate token freshness

---

### 2. Start Simulation

Begin generating synthetic healthcare data.

**Endpoint:**
```
POST /simulate
```

**Authentication:** JWT Bearer Token (Required)

**Request Body:**
```json
{
  "num_providers": 50,
  "num_patients": 100,
  "appointments_per_patient": 3,
  "medical_records_per_patient": 2
}
```

**Request:**
```bash
TOKEN="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."

curl -X POST http://127.0.0.1:8787/simulate \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "num_providers": 50,
    "num_patients": 100,
    "appointments_per_patient": 3,
    "medical_records_per_patient": 2
  }'
```

**Response:** `202 Accepted`
```json
{
  "message": "Simulation started"
}
```

**Status Codes:**
- `202 Accepted` - Simulation started (running asynchronously)
- `400 Bad Request` - Simulation already running
- `401 Unauthorized` - Missing or invalid token
- `500 Internal Server Error` - Failed to start simulation

**Configuration Parameters:**

| Parameter | Type | Default | Range |
|-----------|------|---------|-------|
| `num_providers` | integer | 50 | 1-1000 |
| `num_patients` | integer | 100 | 1-10000 |
| `appointments_per_patient` | integer | 3 | 1-20 |
| `medical_records_per_patient` | integer | 2 | 0-10 |

**Data Generated:**

Simulations generate the following:
- Insurance companies (7 fixed)
- Insurance plans (per company)
- Clinics (10 fixed distribution)
- Providers (configurable count)
- Patients (configurable count)
- Emergency contacts (1 per patient)
- Patient demographics (1 per patient)
- Patient insurance links (random per patient)
- Clinic schedules (per clinic)
- Appointments (configurable per patient)
- Medical records (configurable per patient)

**Performance:**

- Typical generation rate: 1000+ inserts/second
- 100 providers + 100 patients + 300 appointments: ~30-60 seconds
- Non-blocking: Returns immediately, runs asynchronously

**Monitoring:**

Check status while simulation runs:
```bash
curl -X GET http://127.0.0.1:8787/simulate/status \
  -H "Authorization: Bearer $TOKEN"
```

---

### 3. Stop Simulation

Stop the currently running simulation.

**Endpoint:**
```
POST /simulate/stop
```

**Authentication:** JWT Bearer Token (Required)

**Request:**
```bash
TOKEN="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."

curl -X POST http://127.0.0.1:8787/simulate/stop \
  -H "Authorization: Bearer $TOKEN"
```

**Response:** `200 OK`
```json
{
  "message": "Simulation stopped"
}
```

**Status Codes:**
- `200 OK` - Simulation stopped successfully
- `401 Unauthorized` - Missing or invalid token
- `500 Internal Server Error` - Failed to stop simulation

**Behavior:**

- Gracefully stops the simulation task
- Already-inserted data is NOT rolled back
- Safe to call even if no simulation is running
- Can restart immediately after stopping

**Use Cases:**

- Stop long-running simulations
- Free up resources during maintenance
- Change configuration and restart

---

### 4. Get Simulation Status

Check whether a simulation is currently running and view metrics from the last run.

**Endpoint:**
```
GET /simulate/status
```

**Authentication:** JWT Bearer Token (Required)

**Request:**
```bash
TOKEN="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."

curl -X GET http://127.0.0.1:8787/simulate/status \
  -H "Authorization: Bearer $TOKEN"
```

**Response:** `200 OK`
```json
{
  "running": false,
  "last_run": "2024-02-15T14:30:00Z",
  "counts": {
    "insurance_companies": 7,
    "insurance_plans": 42,
    "clinics": 10,
    "providers": 50,
    "patients": 100,
    "appointments": 285,
    "medical_records": 198,
    "emergency_contacts": 100,
    "patient_demographics": 100,
    "patient_insurance": 250,
    "clinic_schedules": 60
  }
}
```

**Status Codes:**
- `200 OK` - Status retrieved successfully
- `401 Unauthorized` - Missing or invalid token
- `500 Internal Server Error` - Failed to retrieve status

**Response Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `running` | boolean | Whether simulation is currently running |
| `last_run` | ISO 8601 datetime | When the last simulation completed (null if never run) |
| `counts` | object | Detailed count of each generated entity type |

**Counts Object:**

| Field | Type | Description |
|-------|------|-------------|
| `insurance_companies` | integer | Number of insurance companies (fixed at 7) |
| `insurance_plans` | integer | Number of insurance plans generated |
| `clinics` | integer | Number of clinics (fixed at 10) |
| `providers` | integer | Number of providers generated |
| `patients` | integer | Number of patients generated |
| `appointments` | integer | Number of appointments generated |
| `medical_records` | integer | Number of medical records generated |
| `emergency_contacts` | integer | Number of emergency contacts (1 per patient) |
| `patient_demographics` | integer | Number of demographic records (1 per patient) |
| `patient_insurance` | integer | Number of patient-insurance relationships |
| `clinic_schedules` | integer | Number of clinic schedule entries |

**Polling Example:**

```bash
#!/bin/bash

TOKEN="your-jwt-token"
ENDPOINT="http://127.0.0.1:8787/simulate/status"

echo "Starting simulation..."
curl -X POST http://127.0.0.1:8787/simulate \
  -H "Authorization: Bearer $TOKEN"

# Poll until complete
while true; do
  STATUS=$(curl -s "$ENDPOINT" -H "Authorization: Bearer $TOKEN")
  RUNNING=$(echo "$STATUS" | jq .running)

  if [ "$RUNNING" = "false" ]; then
    echo "Simulation complete!"
    echo "$STATUS" | jq .counts
    break
  fi

  echo "Still running..."
  sleep 5
done
```

---

### 5. Reset All Data

Delete all generated data by truncating vital_fold schema tables.

**Endpoint:**
```
POST /simulate/reset
```

**Authentication:** JWT Bearer Token (Required)

**Request:**
```bash
TOKEN="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."

curl -X POST http://127.0.0.1:8787/simulate/reset \
  -H "Authorization: Bearer $TOKEN"
```

**Response:** `200 OK`
```json
{
  "message": "All data reset successfully"
}
```

**Status Codes:**
- `200 OK` - Data reset successfully
- `401 Unauthorized` - Missing or invalid token
- `500 Internal Server Error` - Failed to reset data

⚠️ **WARNING: This operation is destructive!**

- Deletes ALL generated data (cannot be undone)
- Insurance companies, clinics are NOT deleted (fixed data)
- Tables truncated in dependency order to maintain referential integrity
- Safe to run even if database is empty

**Tables Truncated:**

1. `medical_records`
2. `appointments`
3. `clinic_schedules`
4. `patient_insurance`
5. `patient_demographics`
6. `emergency_contacts`
7. `patients`
8. `providers`

**Use Cases:**

- Start fresh data generation
- Clean up test data
- Reset before production scenario
- Prepare for new simulation run

**Safe Reset Pattern:**

```bash
# 1. Stop any running simulation
curl -X POST http://127.0.0.1:8787/simulate/stop \
  -H "Authorization: Bearer $TOKEN"

# 2. Wait a moment
sleep 2

# 3. Reset data
curl -X POST http://127.0.0.1:8787/simulate/reset \
  -H "Authorization: Bearer $TOKEN"

# 4. Start new simulation
curl -X POST http://127.0.0.1:8787/simulate \
  -H "Authorization: Bearer $TOKEN"
```

---

## Response Formats

### Success Response

All successful responses include:

```json
{
  "data": { /* response data */ },
  "timestamp": "2024-02-15T14:30:00Z",
  "request_id": "req-123abc-456def"
}
```

### Error Response

All error responses include:

```json
{
  "error": {
    "code": "INVALID_TOKEN",
    "message": "JWT token is invalid or expired",
    "details": "Token signature validation failed"
  },
  "timestamp": "2024-02-15T14:30:00Z",
  "request_id": "req-123abc-456def"
}
```

---

## Error Handling

### HTTP Status Codes

| Code | Meaning | Common Causes |
|------|---------|---------------|
| `200` | OK | Successful request |
| `201` | Created | Resource created successfully |
| `202` | Accepted | Request accepted (async operation) |
| `400` | Bad Request | Invalid input, malformed JSON |
| `401` | Unauthorized | Missing or invalid JWT token |
| `404` | Not Found | Resource doesn't exist |
| `409` | Conflict | Duplicate email or simulation already running |
| `500` | Internal Error | Server error, database failure |

### Error Codes

| Code | HTTP Status | Description |
|------|------------|-------------|
| `INVALID_EMAIL` | 400 | Email format is invalid |
| `INVALID_PASSWORD` | 400 | Password too short or invalid format |
| `SIMULATION_CONFLICT` | 409 | Operation conflicts with running simulation |
| `INVALID_TOKEN` | 401 | JWT token invalid or expired |
| `MISSING_TOKEN` | 401 | Authorization header missing |
| `SIMULATION_RUNNING` | 400 | Cannot start simulation, one already running |
| `DATABASE_ERROR` | 500 | Database connection or query failed |
| `INTERNAL_ERROR` | 500 | Unexpected server error |

### Error Response Example

```bash
curl -X GET http://127.0.0.1:8787/api/v1/me \
  -H "Authorization: Bearer invalid_token"
```

```json
{
  "error": {
    "code": "INVALID_TOKEN",
    "message": "JWT token validation failed",
    "details": "Token signature is invalid"
  },
  "timestamp": "2024-02-15T14:35:22Z",
  "request_id": "req-789xyz-456abc"
}
```

---

## Rate Limiting

Currently, there is **no rate limiting** implemented.

### Planned Rate Limiting

Future versions will implement:

- **Per-user rate limits**: 100 requests per minute
- **Per-endpoint limits**: Simulation endpoints limited to 1 per minute per user
- **Global limits**: 1000 requests per second across all users

Rate limit headers will be included in response:

```http
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 98
X-RateLimit-Reset: 1771895222
```

---

## Best Practices

### Authentication

✅ **Do:**
- Store tokens securely (encrypted in localStorage or secure HTTP-only cookies)
- Include token in `Authorization: Bearer` header
- Refresh token by logging in again before expiration
- Use HTTPS in production (never send tokens over HTTP)

❌ **Don't:**
- Expose tokens in URLs
- Store tokens in localStorage in sensitive apps
- Commit tokens to version control
- Reuse tokens across different applications

### Error Handling

✅ **Do:**
- Check HTTP status code first
- Parse error response for error code and message
- Implement retry logic for 5xx errors
- Log failed requests for debugging

❌ **Don't:**
- Ignore 401 errors (token may have expired)
- Retry indefinitely without backoff
- Expose error details to end users
- Trust only HTTP status (check response body)

### Performance

✅ **Do:**
- Poll `/simulate/status` instead of waiting
- Use appropriate configuration values for data volume
- Cache authentication tokens
- Monitor database connection pool

❌ **Don't:**
- Continuously call endpoints in a loop (use polling with intervals)
- Generate excessive data in single simulation (start smaller)
- Make simultaneous requests for same resource
- Ignore simulation completion (check status before next run)

---

## Swagger/OpenAPI

Interactive API documentation available at:

```
http://127.0.0.1:8787/swagger-ui/
```

Click "Authorize" to enter JWT token, then test endpoints directly from browser.

Raw OpenAPI specification:

```
http://127.0.0.1:8787/api-docs/openapi.json
```


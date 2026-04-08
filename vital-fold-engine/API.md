# VitalFold Engine — API Reference

Complete reference for all 22 REST endpoints. Interactive Swagger UI is also available at `/swagger-ui/` when the server is running.

**Base URL:** `http://localhost:8787` (default)

---

## Authentication

All protected endpoints require a JWT bearer token in the `Authorization` header.

### Get a Token

```bash
curl -X POST http://localhost:8787/api/v1/auth/admin-login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"your-admin-password"}'
```

**Response (200):**
```json
{
  "token": "eyJhbGciOiJIUzI1NiJ9...",
  "user": {
    "id": "00000000-0000-0000-0000-000000000001",
    "email": "admin@admin.internal",
    "created_at": "2026-03-26T12:00:00Z"
  }
}
```

Use the token on all subsequent requests:
```bash
-H "Authorization: Bearer <token>"
```

---

## Public Endpoints

### GET /health

Health check. No authentication required.

```bash
curl http://localhost:8787/health
```

**Response (200):**
```json
{ "status": "ok" }
```

---

### POST /api/v1/auth/login

Login with email and password (database user).

```bash
curl -X POST http://localhost:8787/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"user@example.com","password":"secret"}'
```

**Response (200):**
```json
{
  "token": "eyJhbGciOiJIUzI1NiJ9...",
  "user": { "id": "...", "email": "user@example.com", "created_at": "..." }
}
```

**Errors:**
- `401` — Invalid credentials (same message for wrong email or wrong password)

---

### POST /api/v1/auth/admin-login

Login with admin credentials from environment variables (`ADMIN_USERNAME`, `ADMIN_PASSWORD`). No database user required.

```bash
curl -X POST http://localhost:8787/api/v1/auth/admin-login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"your-admin-password"}'
```

**Response (200):** Same shape as `/login`.

**Errors:**
- `401` — Invalid admin credentials

---

## User Endpoints

### GET /api/v1/me

Get the current user's profile from the JWT claims.

```bash
curl http://localhost:8787/api/v1/me \
  -H "Authorization: Bearer <token>"
```

**Response (200):**
```json
{
  "id": "00000000-0000-0000-0000-000000000001",
  "email": "admin@admin.internal",
  "created_at": "2026-03-26T12:00:00Z"
}
```

---

## Population Endpoints (Phase 1 & 2)

These endpoints seed Aurora DSQL with synthetic healthcare data. No DynamoDB writes.

### POST /populate

Legacy endpoint — runs all 13 populate steps in a single call. Prefer the split `/populate/static` + `/populate/dynamic` workflow for production.

```bash
curl -X POST http://localhost:8787/populate \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "plans_per_company": 3,
    "providers": 50,
    "patients": 50000,
    "records_per_appointment": 1,
    "start_date": "2026-04-01",
    "end_date": "2026-06-30",
    "clinic_weights": [12, 3, 14, 14, 2, 14, 14, 12, 8, 8]
  }'
```

All fields are optional — omit any to use defaults. Body can be omitted entirely.

**Response (202):**
```json
{ "message": "Population started" }
```

**Errors:**
- `400` — Invalid date range (end before start, or > 90 days)
- `409` — A run is already in progress

---

### POST /populate/static

**Phase 1:** Seed reference data only (insurance, clinics, providers, patients, demographics, insurance links). Run once. Returns `409` if static data already exists.

```bash
curl -X POST http://localhost:8787/populate/static \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"patients": 1000, "providers": 20}'
```

All fields optional. Defaults: `plans_per_company=3`, `providers=50`, `patients=50000`.

**Response (202):**
```json
{ "message": "Static populate started (8 steps)" }
```

**Errors:**
- `409` — Static data already exists (patients > 0), or a run is already in progress

---

### POST /populate/dynamic

**Phase 2:** Seed date-dependent data (clinic schedules, appointments, medical records, patient visits, patient vitals) for a date range. Requires Phase 1 data to exist. Can be called multiple times for different date ranges.

```bash
curl -X POST http://localhost:8787/populate/dynamic \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "start_date": "2026-04-01",
    "end_date": "2026-04-30",
    "records_per_appointment": 1,
    "clinic_weights": [12, 3, 14, 14, 2, 14, 14, 12, 8, 8]
  }'
```

`start_date` and `end_date` are **required**. Other fields optional.
Appointment volume is auto-calculated: providers × 36 slots/day, distributed by clinic weights.

**Response (202):**
```json
{ "message": "Dynamic populate started for 2026-04-01 to 2026-04-30 (7 steps)" }
```

**Errors:**
- `400` — No static data found, date range > 90 days, end before start, or date overlap with existing data
- `409` — A run is already in progress

---

### GET /populate/dates

List all dates that have been populated with appointments (useful to avoid overlap).

```bash
curl http://localhost:8787/populate/dates \
  -H "Authorization: Bearer <token>"
```

**Response (200):**
```json
["2026-04-01", "2026-04-02", "2026-04-03"]
```

---

### POST /populate/reset-dynamic

Delete dynamic data only (schedules, appointments, records, visits, vitals). Preserves static reference data (insurance, clinics, providers, patients).

```bash
curl -X POST http://localhost:8787/populate/reset-dynamic \
  -H "Authorization: Bearer <token>"
```

**Response (202):**
```json
{ "message": "Dynamic data reset started" }
```

**Errors:**
- `409` — A run is already in progress

---

## Simulation Endpoints (Phase 3 — DynamoDB Sync)

These endpoints sync Aurora visit data to DynamoDB.

### POST /simulate

Sync today's Aurora visits to both DynamoDB tables (`patient_visit` and `patient_vitals`).

```bash
curl -X POST http://localhost:8787/simulate \
  -H "Authorization: Bearer <token>"
```

**Response (202):**
```json
{ "message": "Simulation started" }
```

**Errors:**
- `409` — A run is already in progress

---

### POST /simulate/date-range

Sync Aurora visit data to DynamoDB for a specific date range. Requires Dynamic Populate to have created visits for those dates.

```bash
curl -X POST http://localhost:8787/simulate/date-range \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"start_date": "2026-04-01", "end_date": "2026-04-30"}'
```

**Response (202):**
```json
{ "message": "DynamoDB sync started for 2026-04-01 to 2026-04-30" }
```

**Errors:**
- `400` — No visits exist for the date range, or range > 90 days
- `409` — A run is already in progress

---

### POST /simulate/stop

Stop any running background task (populate, simulate, reset, timelapse).

```bash
curl -X POST http://localhost:8787/simulate/stop \
  -H "Authorization: Bearer <token>"
```

**Response (200):**
```json
{ "message": "Run stopped" }
```

---

### GET /simulate/status

Poll the current run status, row counts, and progress. This is the primary polling endpoint.

```bash
curl http://localhost:8787/simulate/status \
  -H "Authorization: Bearer <token>"
```

**Response (200):**
```json
{
  "running": true,
  "last_run": "2026-03-26T15:30:00Z",
  "insurance_companies": 7,
  "insurance_plans": 21,
  "clinics": 10,
  "providers": 50,
  "patients": 50000,
  "emergency_contacts": 50000,
  "patient_demographics": 50000,
  "patient_insurance": 50000,
  "clinic_schedules": 250,
  "appointments": 100000,
  "no_shows": 1000,
  "cancellations": 9000,
  "medical_records": 90000,
  "patient_visits": 90000,
  "patient_vitals": 90000,
  "surveys": 27000,
  "cpt_codes": 12,
  "appointment_cpt": 108000,
  "dynamo_patient_visits": 3000,
  "dynamo_patient_vitals": 3000,
  "populate_progress": {
    "current_step": "Appointments",
    "steps_done": 9,
    "total_steps": 13,
    "rows_written": 150271,
    "is_complete": false
  }
}
```

The `populate_progress`, `reset_progress`, and `dynamo_progress` fields appear only when a corresponding operation is active.

---

### GET /simulate/db-counts

Live record counts queried directly from Aurora (13 `COUNT(*)` queries) and DynamoDB (scan with `Select::Count`). More expensive than `/status` — use on explicit user action, not polling.

```bash
curl http://localhost:8787/simulate/db-counts \
  -H "Authorization: Bearer <token>"
```

**Response (200):** Same shape as the counts in `/status`, but with live database values instead of in-memory counters.

---

## Reset Endpoints

### POST /simulate/reset

Delete all data from all 13 Aurora DSQL tables. FK-safe deletion order with retry logic for Aurora DSQL `OC000` errors.

```bash
curl -X POST http://localhost:8787/simulate/reset \
  -H "Authorization: Bearer <token>"
```

**Response (202):**
```json
{ "message": "Aurora reset started" }
```

Poll `/simulate/status` — the `reset_progress` field tracks which table is being deleted.

**Errors:**
- `409` — A run is already in progress

---

### POST /simulate/reset-dynamo

Delete all items from both DynamoDB tables (`patient_visit` and `patient_vitals`). Uses scan + batch delete with throttle pacing.

```bash
curl -X POST http://localhost:8787/simulate/reset-dynamo \
  -H "Authorization: Bearer <token>"
```

**Response (202):**
```json
{ "message": "DynamoDB reset started" }
```

**Errors:**
- `409` — A run is already in progress

---

## Visualization Endpoints

### POST /simulate/timelapse

Start an hour-by-hour heatmap animation for populated dates. Steps through 8 AM to 5 PM, updating per-clinic appointment counts. Auto-populates DynamoDB if needed.

```bash
curl -X POST http://localhost:8787/simulate/timelapse \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"window_interval_secs": 5}'
```

`window_interval_secs` is optional (default: 5). Controls seconds between hour-window updates.

**Response (202):**
```json
{ "message": "Timelapse started" }
```

---

### GET /simulate/heatmap

Poll the current heatmap state during a timelapse or replay.

```bash
curl http://localhost:8787/simulate/heatmap \
  -H "Authorization: Bearer <token>"
```

**Response (200) — active:**
```json
{
  "simulation_day": "2026-04-15",
  "day_number": 3,
  "total_days": 30,
  "sim_hour": 14,
  "clinics": [
    { "clinic_id": "...", "city": "Miami", "state": "FL", "active_patients": 12 },
    { "clinic_id": "...", "city": "Atlanta", "state": "GA", "active_patients": 8 }
  ]
}
```

**Response (200) — inactive:**
```json
{ "active": false }
```

---

### POST /simulate/replay

Start a read-only heatmap replay using Aurora data only. No DynamoDB writes. Same visualization as timelapse but without side effects.

```bash
curl -X POST http://localhost:8787/simulate/replay \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"window_interval_secs": 3}'
```

**Response (202):**
```json
{ "message": "Replay started" }
```

---

### POST /simulate/replay-reset

Clear the heatmap replay state. No data is deleted.

```bash
curl -X POST http://localhost:8787/simulate/replay-reset \
  -H "Authorization: Bearer <token>"
```

**Response (200):**
```json
{ "message": "Replay state cleared" }
```

---

### GET /simulate/visitors

Get today's visitors (patient names) grouped by clinic.

```bash
curl http://localhost:8787/simulate/visitors \
  -H "Authorization: Bearer <token>"
```

**Response (200):**
```json
{
  "date": "2026-03-26",
  "clinics": [
    {
      "clinic_id": "...",
      "city": "Miami",
      "state": "FL",
      "visitors": [
        { "first_name": "John", "last_name": "Smith", "appointment_time": "2026-03-26T09:30:00" },
        { "first_name": "Jane", "last_name": "Doe", "appointment_time": "2026-03-26T10:15:00" }
      ]
    }
  ]
}
```

---

## Admin Endpoints

### POST /admin/init-db

**Destructive.** Drops the entire `vital_fold` schema (losing all simulation data) and recreates all 16 tables from `migrations/init.sql`. The `public.users` auth table is preserved (uses `CREATE TABLE IF NOT EXISTS`). In-memory simulation counts are reset.

The SQL file is embedded into the binary at compile time via `include_str!`, so no filesystem access is required at runtime. Each statement is parsed and executed individually.

```bash
curl -X POST http://localhost:8787/admin/init-db \
  -H "Authorization: Bearer <token>"
```

**Response (200):**
```json
{ "message": "Schema initialized — 42 SQL statements executed" }
```

**Errors:**
- `401` — Unauthorized
- `500` — SQL execution failed (check server logs for the specific statement)

The admin dashboard includes an "Init Database" button with a confirmation modal that invokes this endpoint.

---

## Error Reference

All errors return JSON with an `error` field:

```json
{ "error": "Description of what went wrong" }
```

| Status | Meaning | Common Causes |
|--------|---------|---------------|
| `400` | Bad Request | Invalid date range, missing required fields, date overlap |
| `401` | Unauthorized | Missing/expired/invalid JWT token, wrong credentials |
| `404` | Not Found | User not found in database |
| `409` | Conflict | A background task is already running, static data already exists |
| `500` | Internal Server Error | Database connection failure, unexpected error |

---

## Polling Pattern

All long-running operations (populate, simulate, reset) follow the same pattern:

1. **POST** to start the operation — receive `202 Accepted`
2. **Poll** `GET /simulate/status` every 1-2 seconds
3. Check `running == false` to know when complete
4. Read progress fields (`populate_progress`, `reset_progress`, `dynamo_progress`) for real-time updates
5. Read count fields to see final row counts

# Build History — Initial Implementation (2026-02-22)

> Consolidated from: CODE_REVIEW_SUMMARY.md, CODE_VALIDATION_REPORT.md,
> FIXES_TRACKING.md, MISTAKE_ANALYSIS.md

---

## Overview

First major build completed 2026-02-22: 40+ files, ~12,800 lines across 7 modules.
Built by Claude Haiku 4.5 from the `claude.md` specification.

---

## Critical Fixes Applied (5)

All discovered during initial code review, all resolved before first working build.

### Fix 1: User Handler Extractor (CRITICAL)

**Issue:** `me` handler used `web::Json<Claims>` — tries to deserialize Claims from
request body instead of request extensions where JWT middleware inserts them.

**Root cause:** Misunderstanding of Actix Web extractors. `web::Json<T>` reads the
body; `req.extensions().get::<T>()` reads middleware-injected data.

**Fix:** Changed to `HttpRequest` parameter with `req.extensions().get::<Claims>()`.

**Lesson learned:** Always trace data flow — where does it originate, how does it
get there, and which extractor or method accesses it. When unsure about framework
behavior, read the documentation first, never guess.

### Fix 2: Database Pool Type Conflict (CRITICAL)

**Issue:** Mixed `deadpool-postgres::Pool` and `sqlx::PgPool` types.
`sqlx::query!` macros require `sqlx::PgPool`, not `deadpool_postgres::Pool`.

**Fix:** Unified to `sqlx::PgPool` everywhere. Removed deadpool dependency.

### Fix 3: Input Validation (HIGH)

**Issue:** No email format validation, no password minimum length, empty fields accepted.

**Fix:** Added `validate()` methods to `RegisterRequest` and `LoginRequest`.

### Fix 4: Email Uniqueness Race Condition (HIGH)

**Issue:** TOCTOU — SELECT to check existence, then INSERT. Concurrent requests
could both pass the SELECT and collide on INSERT.

**Fix:** Removed SELECT check. Let INSERT fail on UNIQUE constraint, catch and
return 400.

### Fix 5: Login Empty Field Rejection (MEDIUM)

**Issue:** Login accepted empty or whitespace-only email/password without error.

**Fix:** Added validation to `LoginRequest`.

---

## Generator Validation Summary

All 6 generators validated against `claude.md` specification. Key checks:

| Generator | Validation |
|---|---|
| `insurance.rs` | 7/7 companies match exactly (Orange Spear, Care Medical, Cade Medical, Multiplied Health, Octi Care, Tatnay, Caymana). 16 plans distributed 3+2+3+2+2+2+3. |
| `clinic.rs` | 10/10 locations match. FL: 5 clinics (Miami 2, Jacksonville 2, Orlando 1, Tallahassee 1), GA: 2, NC: 2. Hours: 9am-5pm Mon-Fri. |
| `provider.rs` | 8 cardiac-focused specialties. 2 license types (MD, DO). Names via `fake` crate. |
| `patient.rs` | DOB: 18-80 years old. Emergency contacts: 1:1. Demographics: gender (M/F/Other). Insurance: 1-3 plans per patient. |
| `appointment.rs` | 5 cardiac-appropriate reasons. Hours: 9am-5pm. Scheduling: 1-90 days ahead. |
| `medical_record.rs` | 8/8 diagnoses match exactly (AFib, CAD, Chest Pain, Hypertension, Hyperlipidemia, SOB, Tachycardia, Bradycardia). Treatment pairing correct. |

### Cross-Domain Checks

- FK references properly maintained through `SimulationContext`
- UUID propagation correct through all stages
- Count accumulation accurate
- All parameterized queries (no SQL injection)
- BigDecimal for monetary values, DateTime<Utc> for timestamps

---

## Handler & Route Validation

| Endpoint | Method | Status Code | Notes |
|---|---|---|---|
| `/simulate` | POST | 202 | Background task via `tokio::spawn`, `try_start()` prevents concurrent runs |
| `/simulate/stop` | POST | 200 | Sets running flag to false |
| `/simulate/status` | GET | 200 | Returns running flag, last_run, counts |
| `/simulate/reset` | POST | 200 | TRUNCATE 11 tables in dependency order with CASCADE |

TRUNCATE order: medical_record -> appointment -> clinic_schedule -> patient_insurance ->
patient_demographics -> emergency_contact -> patient -> provider -> clinic ->
insurance_plan -> insurance_company.

---

## Compilation Notes

Initial build had 5 type system issues (not logic errors):
1. BigDecimal serde feature flag
2. StreetAddress faker import path
3. AWS SDK endpoint method signature
4. Actix middleware return type
5. SimulatorState Arc wrapping

All resolved during production build step (commit `087ee32`).

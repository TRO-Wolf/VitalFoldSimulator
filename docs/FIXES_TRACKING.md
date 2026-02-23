# Critical Fixes Tracking

## Overview
This document tracks the 5 critical production-grade issues discovered during code review.

---

## Fix #1: Correct user.rs Handler (CRITICAL) ✅

**Status**: Complete
**Priority**: CRITICAL - Endpoint will fail
**File**: `src/handlers/user.rs`

### Issue
The `me` handler tries to extract Claims from JSON body (`web::Json<Claims>`) instead of from request extensions. The jwt_validator middleware inserts Claims into extensions, not the request body.

### Current Code
```rust
pub async fn me(
    pool: web::Data<DbPool>,
    claims: web::Json<Claims>,  // WRONG!
) -> Result<HttpResponse, AppError>
```

### Fix Required
Replace with extraction from HttpRequest extensions.

### Test Plan
- [ ] Handler compiles
- [ ] Test with valid JWT: GET /api/v1/me with bearer token → 200 OK
- [ ] Test without token: GET /api/v1/me → 401 Unauthorized
- [ ] Test with invalid token: GET /api/v1/me with bad bearer token → 401 Unauthorized

---

## Fix #2: Resolve Database Pool Type Conflict (CRITICAL) ✅

**Status**: Complete
**Priority**: CRITICAL - Code won't compile or work
**Files**:
- `src/db/mod.rs`
- `src/handlers/auth.rs`
- `src/handlers/user.rs`

### Issue
Using `deadpool_postgres::Pool` (DbPool) with `sqlx::query!` and `sqlx::query_as` macros. These are incompatible types.

### Root Cause
- `sqlx::query!` expects `sqlx::PgPool`
- `deadpool_postgres::Pool` is different type
- Both can't be used together

### Solution Options
**Option A (Recommended)**: Use BOTH pools
- `sqlx::PgPool` for auth handlers (public.users)
- Keep `sqlx::PgPool` for simulation generators (vital_fold.*)
- Simplifies to single type

**Option B**: Use only sqlx::PgPool everywhere
- Remove deadpool dependency
- Update Cargo.toml
- Simpler architecture

### Test Plan
- [ ] `cargo check` passes
- [ ] All handlers compile
- [ ] Database queries execute successfully

---

## Fix #3: Add Input Validation (HIGH) ✅

**Status**: Complete
**Priority**: HIGH - Security issue
**File**: `src/handlers/auth.rs` + `src/models/user.rs`

### Issues
1. No email format validation
2. No password minimum length
3. Empty fields not rejected

### Changes Required
- Add validation method to RegisterRequest
- Add validation method to LoginRequest
- Call validation in handlers

### Test Plan
- [ ] Invalid email rejected: "test" → 400 Bad Request
- [ ] Too short password rejected: "pass" → 400 Bad Request
- [ ] Valid registration accepted: "user@example.com" + "password1234" → 201 Created
- [ ] Empty email rejected: "" → 400 Bad Request
- [ ] Empty password rejected: "" → 400 Bad Request

---

## Fix #4: Handle Email Uniqueness Properly (HIGH) ✅

**Status**: Complete
**Priority**: HIGH - Race condition vulnerability
**File**: `src/handlers/auth.rs`

### Issue
TOCTOU (Time-of-Check to Time-of-Use) vulnerability:
1. Check if email exists with SELECT
2. If not, INSERT new user
3. Between check and insert, another request could insert same email

### Solution
Let database handle uniqueness constraint, catch violation:
- Remove separate SELECT query
- Let INSERT fail with constraint violation
- Catch error and return 400 Bad Request

### Test Plan
- [ ] Single registration with unique email: 201 Created
- [ ] Duplicate registration attempt: 400 Bad Request "Email already registered"
- [ ] Concurrent registrations with same email: Only one succeeds (requires load test)

---

## Fix #5: Add Password Validation to LoginRequest (MEDIUM) ✅

**Status**: Complete
**Priority**: MEDIUM - Input hygiene
**File**: `src/handlers/auth.rs` + `src/models/user.rs`

### Issue
Login accepts empty or whitespace-only fields without error.

### Changes Required
- Add validate() method to LoginRequest
- Reject empty email
- Reject empty password
- Call validation in login handler

### Test Plan
- [ ] Empty email rejected: "" → 400 Bad Request
- [ ] Whitespace-only email rejected: "   " → 400 Bad Request
- [ ] Empty password rejected: "" → 400 Bad Request
- [ ] Valid login accepted: "user@example.com" + "password" → 200 OK

---

## Execution Order

1. **Fix #1**: Correct user.rs Handler
2. **Fix #2**: Resolve Database Pool Type
3. **Fix #3**: Add Input Validation (RegisterRequest)
4. **Fix #4**: Handle Email Uniqueness
5. **Fix #5**: Add Password Validation (LoginRequest)

After each fix:
- Run `cargo check`
- Verify no new errors introduced
- Mark as complete

---

## Verification Checklist

After all fixes:
- [x] `cargo check` passes
- [x] `cargo clippy` has no warnings
- [x] All handlers compile
- [ ] Unit tests pass
- [ ] Can proceed to Steps 10-21

---

## Notes

**Database Pool Decision**: Need to decide between Option A (both pools) vs Option B (single sqlx pool) for Fix #2.

**Recommendation**: Option B (single sqlx pool) is simpler but requires removing deadpool dependency from Cargo.toml.

# Deprecation & Optimization History (2026-03-07)

> Consolidated from: DEPRECATION_UPDATE_PLAN.md, DEPRECATION_UPDATE_COMPLETION.md
> Executed by Claude Haiku. Committed as `ae104e0`.

---

## Summary

Removed 5 unused dependencies, fixed a JWT security vulnerability, replaced all
deprecated `chrono::Duration` usage with `chrono::TimeDelta`, and synced `claude.md`
to match the actual sqlx-based implementation.

---

## Changes Made

### 1. Removed Unused Dependencies from Cargo.toml

| Dependency | Why removed |
|---|---|
| `tokio-postgres 0.7` | Never imported; sqlx used exclusively |
| `duckdb 1.4.4` | Never imported; massive bundled binary (multi-minute build) |
| `polars 0.53` | Never imported; large data science library |
| `config 0.15` | Never imported; config via dotenvy + std::env |
| `aws-sdk-rds 1` | Never imported; only dsql & dynamodb used |

Impact: ~2,800 lines removed from Cargo.lock, significant build time improvement.

### 2. JWT Algorithm Pinning (Security Fix)

**File:** `src/middleware/auth.rs`

`Validation::default()` accepted any signing algorithm, enabling algorithm confusion
attacks (CVE-2015-9235 class). Changed to `Validation::new(Algorithm::HS256)` to pin
validation to exactly the algorithm used during encoding.

### 3. chrono::Duration -> chrono::TimeDelta (8 occurrences)

`chrono::Duration` is a deprecated alias for `TimeDelta` since chrono 0.4.32.

| File | Occurrences |
|---|---|
| `src/middleware/auth.rs` | 1 (`Duration::hours`) |
| `src/generators/appointment.rs` | 5 (`Duration::days`, `Duration::minutes`) |
| `src/generators/patient.rs` | 2 (`Duration::days`) |
| `src/generators/medical_record.rs` | 1 (`Duration::minutes`) |

### 4. Removed Orphaned Error Impl

`errors.rs` had a `From<tokio_postgres::Error>` impl for a dependency that no longer
existed. Removed 6 lines.

### 5. Synced claude.md to Implementation

Updated 5 sections to reflect sqlx (not deadpool-postgres):
- Tech stack table
- `src/errors.rs` module spec
- `src/db/mod.rs` module spec
- JWT validation description
- Cargo.toml dependency list and common imports

---

## Verification

All passed:
- `cargo check` — 0 errors
- `grep "chrono::Duration" src/` — no matches
- `grep "Validation::default" src/` — no matches
- `grep "tokio-postgres\|deadpool-postgres\|duckdb\|polars\|aws-sdk-rds" Cargo.toml` — no matches

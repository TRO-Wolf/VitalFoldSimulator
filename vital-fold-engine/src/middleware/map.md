# `vital-fold-engine/src/middleware/` — map

**Purpose:** Request middleware. Currently just JWT bearer-token validation.

## Contents

| File | What it is |
|---|---|
| auth.rs | `jwt_validator` bearer middleware; token generation + validation helpers; `Claims` struct. Holds the crate's unit tests for auth |
| mod.rs | Re-export of `auth` |

## I want to…

| I want to… | Go to |
|---|---|
| Change which routes are public | [../routes.rs](../routes.rs) (middleware is wrapped per-scope there) |
| Change token lifetime / claims | auth.rs (reads `JWT_SECRET`, `JWT_EXPIRY_HOURS` from [../config.rs](../config.rs)) |
| Add an auth test | auth.rs `#[cfg(test)] mod tests` |

## Pointers

- Parent: [src/](../map.md).
- Public routes (no token): `/`, `/health`, `/api/v1/auth/login`, `/admin-login`, Swagger UI, static assets.

## Debug

- 401 on a protected route → missing/expired bearer token or `JWT_SECRET` mismatch between issue and validate.
- HS256 only; secret must be ≥32 chars or startup config validation fails.

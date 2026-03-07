# `src/middleware/` — Claude Context

> **Purpose:** Self-contained reference for the `src/middleware/` subdirectory. Two files: `mod.rs` (re-export only) and `auth.rs` (all JWT logic). Covers token generation, validation, and Actix middleware integration.

---

## Overview

The middleware module provides JWT-based authentication for all protected routes. It has three responsibilities:

1. **`generate_token`** — mint a signed JWT for a user after successful login/register
2. **`validate_token`** — decode and verify a JWT, returning `Claims` or an error
3. **`jwt_validator`** — Actix middleware function that guards protected route scopes

The `Claims` struct is the shared identity type that flows from middleware into handlers via Actix's request extensions.

---

## `mod.rs`

```rust
pub mod auth;
pub use auth::*;
```

Simple re-export. All types and functions are accessed as `crate::middleware::auth::*` or via the glob.

---

## `Claims` Struct

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,    // User ID as string (UUID.to_string())
    pub email: String,  // User email
    pub exp: i64,       // Expiration unix timestamp
    pub iat: i64,       // Issued-at unix timestamp
}
```

**Notes:**
- `sub` stores the user's UUID as a `String` — parse with `Uuid::parse_str(&claims.sub)` in handlers
- For admin tokens, `sub` is `"00000000-0000-0000-0000-000000000001"` (hardcoded stable UUID)
- Inserted into Actix request extensions by `jwt_validator`; extracted by handlers via `req.extensions().get::<Claims>()`

---

## Public Functions

### `generate_token`

```rust
pub fn generate_token(
    user_id: Uuid,
    email: String,
    cfg: &Config,
) -> Result<String, AppError>
```

**What it does:**
1. Gets current UTC time
2. Computes expiry as `now + cfg.jwt_expiry_hours`
3. Builds `Claims { sub: user_id.to_string(), email, iat: now.timestamp(), exp: expiry.timestamp() }`
4. Encodes with `jsonwebtoken::encode(&Header::default(), &claims, &EncodingKey::from_secret(cfg.jwt_secret.as_ref()))`
5. `Header::default()` uses HS256 algorithm

**On error:** Logs with `tracing::error!`, returns `AppError::Internal`.

**Called from:** `handlers/auth.rs` (register, login, admin_login)

---

### `validate_token`

```rust
pub fn validate_token(token: &str, secret: &str) -> Result<Claims, AppError>
```

**What it does:**
1. Creates `DecodingKey::from_secret(secret.as_ref())`
2. Calls `decode::<Claims>(token, &decoding_key, &Validation::new(Algorithm::HS256))`
3. Returns `data.claims` on success

**On error:** Logs with `tracing::warn!`, returns `AppError::Unauthorized("Invalid or expired token")`.

**Called from:** `jwt_validator` (below) and can be called directly from handlers if needed.

---

### `jwt_validator`

```rust
pub async fn jwt_validator(
    mut req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, (Error, ServiceRequest)>
```

**Actix middleware signature** — used with `HttpAuthentication::bearer(jwt_validator)`.

**What it does:**
1. Extracts `Config` from `req.app_data::<web::Data<Config>>()`
   - If missing → returns `ErrorInternalServerError` (500)
2. Calls `validate_token(credentials.token(), &cfg.jwt_secret)`
   - On success → inserts `Claims` into `req.extensions_mut()`, returns `Ok(req)`
   - On failure → logs at debug level, returns `ErrorUnauthorized` (401)

**How to apply to a route scope (from `routes.rs`):**
```rust
web::scope("/api/v1")
    .wrap(HttpAuthentication::bearer(jwt_validator))
    .route("/me", web::get().to(user::me))
```

**How handlers access Claims:**
```rust
// In any protected handler:
let claims = req.extensions().get::<Claims>()
    .ok_or_else(|| AppError::Unauthorized("Authentication required".to_string()))?
    .clone();
let user_id = Uuid::parse_str(&claims.sub)?;
```

---

## Security Notes

| Concern | Implementation |
|---|---|
| Algorithm | HS256 (shared secret) |
| Secret source | `cfg.jwt_secret` (env var `JWT_SECRET`, min 32 chars) |
| Expiry | Configurable via `cfg.jwt_expiry_hours` (default 24h) |
| Expiry enforcement | `jsonwebtoken` library validates `exp` claim automatically |
| Same error for bad user + wrong password | Enforced in `handlers/auth.rs`, not middleware |
| Admin stable identity | UUID `00000000-0000-0000-0000-000000000001` — no DB row needed |

---

## Cross-Module Relationships

**Imports from:**
- `crate::config::Config`
- `crate::errors::AppError`
- `jsonwebtoken`, `chrono`, `uuid`, `actix_web`, `actix_web_httpauth`, `serde`

**Exported to:**
- `handlers/auth.rs` — imports `generate_token()`
- `handlers/user.rs` — imports `Claims` (extracts from request extensions)
- `routes.rs` — imports `jwt_validator` to wrap protected scopes
- `main.rs` — imports `jwt_validator`

---

## Common Imports for This Module

```rust
use crate::config::Config;
use crate::errors::AppError;
use actix_web::{dev::ServiceRequest, Error, HttpMessage};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
```

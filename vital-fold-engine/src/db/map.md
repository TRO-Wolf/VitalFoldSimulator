# `vital-fold-engine/src/db/` — map

**Purpose:** Aurora DSQL connection-pool setup and IAM auth-token lifecycle.

## Contents

| File | What it is |
|---|---|
| mod.rs | `DbPool` type alias + pool creation; spawns a background task that refreshes the DSQL IAM auth token before it expires (~15 min) |

## I want to…

| I want to… | Go to |
|---|---|
| Change pool size / connect options | mod.rs (reads `DB_POOL_SIZE` from [config.rs](../config.rs)) |
| Understand DSQL token auth | mod.rs token-refresh task + [../../INSTALLATION.md](../../INSTALLATION.md) § AWS Prerequisites |

## Pointers

- Parent: [src/](../map.md).
- Token signing uses `aws-sdk-dsql`; region comes from `DSQL_REGION`.

## Debug

- `Failed to create database pool` at startup → AWS creds not found or IAM principal lacks `dsql:DbConnectAdmin`.
- Repeated token-refresh errors in logs → STS unreachable or system clock skew (signature validation is strict).

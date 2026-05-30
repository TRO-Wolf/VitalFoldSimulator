# `vital-fold-engine/migrations/` — map

**Purpose:** Database schema as a single idempotent DDL file. Not run by `cargo sqlx migrate` — executed at runtime by `POST /admin/init-db`, which `include_str!`s this file and runs each statement.

## Contents

| File | What it is |
|---|---|
| init.sql | Full DDL: `DROP SCHEMA vital_fold CASCADE` + recreate all 16 `vital_fold.*` tables, plus `public.users`. Seeds 12 CPT codes. |

## I want to…

| I want to… | Go to |
|---|---|
| Add or alter a table | Edit `init.sql`, then mirror it in [../../docs/models-spec.md](../../docs/models-spec.md) and the relevant [../src/models/](../src/map.md) struct |
| See how the schema is applied | [../src/handlers/](../src/handlers/map.md) → `simulation::init_database` |
| Understand intentional data-quality traps | [../../CLAUDE.md](../../CLAUDE.md) §7.4 |

## Pointers

- Parent: [crate root](../map.md).
- This file is the **schema source of truth**; `docs/models-spec.md` and `docs/database` renders describe it but the SQL wins.

## Debug

- `init-db` is destructive (`DROP SCHEMA ... CASCADE`) — it wipes all simulation data. Confirm intent before calling.
- Aurora DSQL quirk: `CREATE INDEX ASYNC` briefly holds a schema lock; `init_database` retries OC000/OC001 with backoff.
- DSQL has no FK constraints by design — referential integrity is enforced at the application layer.

# `docs/` — map

**Purpose:** Supporting reference documentation that doesn't live inside the crate — data-model specs, the DynamoDB schema, frontend architecture, orchestration examples, plus archived history.

## Contents

| File | What it is |
|---|---|
| [models-spec.md](models-spec.md) | Rust struct ↔ table mapping for every Aurora table |
| [dynamo.md](dynamo.md) | DynamoDB schema, key design, write strategy, TTL |
| dynamo.json | Companion sample item shapes for `dynamo.md` |
| [frontend.md](frontend.md) | Preact dashboard architecture (components, pages, state) |
| [airflow-integration.md](airflow-integration.md) | Example DAGs for orchestrating the three populate phases |
| [history/](history/map.md) | Archived historical notes (not load-bearing) |

## I want to…

| I want to… | Go to |
|---|---|
| Map a Rust struct to its table | [models-spec.md](models-spec.md) |
| Understand DynamoDB writes / TTL | [dynamo.md](dynamo.md) |
| Work on the admin SPA | [frontend.md](frontend.md) + [../vital-fold-engine/static/](../vital-fold-engine/static/map.md) |
| Schedule populate/sync jobs | [airflow-integration.md](airflow-integration.md) |
| Read how the project came to be | [history/](history/map.md) |

## Pointers

- Parent: [repo root](../map.md).
- Schema source of truth is [../vital-fold-engine/migrations/init.sql](../vital-fold-engine/migrations/init.sql); these docs describe it but the SQL wins.

## Debug

- Doc drift: when schema, models, or the frontend change, update the matching file here in the same change (CLAUDE.md §9).
- `dynamo.md` and `dynamo.json` describe the same item shape — keep them consistent.

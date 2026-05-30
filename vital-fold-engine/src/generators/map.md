# `vital-fold-engine/src/generators/` — map

**Purpose:** Synthetic data builders — one module per domain. Orchestrated by `mod.rs`, which sequences the populate phases and bulk-inserts via `UNNEST` (2,500-row batches).

## Contents

| File | What it is |
|---|---|
| mod.rs | `SimulationContext`, `run_populate_static` (8 steps) / `run_populate_dynamic` (7 steps), step-name constants, `hydrate_counts_from_db`, weight distribution helpers |
| insurance.rs | 7 fixed carriers + insurance plans |
| clinic.rs | 10 fixed SE-US clinics + clinic schedules |
| provider.rs | ~50 providers distributed across clinics |
| patient.rs | Patients + emergency contacts + demographics + insurance links |
| appointment.rs | Appointments on a slot grid; stochastic status (completed/no_show/cancelled) |
| medical_record.rs | Medical records (completed appointments only) |
| visit.rs | `patient_visit` + `patient_vitals`; weekday-aware wait-time (Mon/Tue backlog) |
| survey.rs | Surveys for ~30% of visits |
| rvu.rs | `appointment_cpt` billing line-items with RVU snapshots |

## I want to…

| I want to… | Go to |
|---|---|
| Change populate step order / counts | mod.rs (step constants + run_* functions) |
| Tune data-quality traps or wait-time tail | visit.rs (rate constants + `provider_seen_offset_minutes`) |
| Change appointment status rates | appointment.rs (`NO_SHOW_RATE`, `CANCELLATION_RATE`) |
| Add a new generated table | New `*.rs` here + wire a step in mod.rs + add to [../models/](../models/map.md) |

## Pointers

- Parent: [src/](../map.md).
- House style: section-banner doc comments (see [appointment.rs](appointment.rs) and CLAUDE.md house-style notes).

## Debug

- Only `status = 'completed'` appointments produce downstream rows (CLAUDE.md §7.3) — empty visits/vitals usually means everything rolled no_show/cancelled.
- `!Send` RNG: build all rows synchronously, drop `rng` before any `.await` (CLAUDE.md §7.5).
- Bulk inserts cap at 2,500 rows/statement (DSQL limit, CLAUDE.md §7.6).

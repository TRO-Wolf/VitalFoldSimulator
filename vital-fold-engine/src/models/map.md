# `vital-fold-engine/src/models/` — map

**Purpose:** `sqlx::FromRow` + `serde` structs mapping 1:1 to database tables. Pure data contracts — no business logic.

## Contents

| File | Struct(s) |
|---|---|
| user.rs | `User` (public.users) |
| insurance.rs | `InsuranceCompany`, `InsurancePlan` |
| clinic.rs | `Clinic`, `ClinicSchedule` |
| provider.rs | `Provider` |
| patient.rs | `Patient`, `EmergencyContact`, `PatientDemographics` |
| appointment.rs | `Appointment` |
| medical_record.rs | `MedicalRecord` |
| patient_visit.rs | `PatientVisit`, `PatientVisitWithVitals` |
| patient_vital.rs | `PatientVital` |
| survey.rs | `Survey` |
| mod.rs | Module re-exports |

## I want to…

| I want to… | Go to |
|---|---|
| Add a field to a table's struct | Matching `*.rs` here + [../../migrations/init.sql](../../migrations/map.md) + [../../../docs/models-spec.md](../../../docs/models-spec.md) |
| See the canonical struct↔table mapping | [../../../docs/models-spec.md](../../../docs/models-spec.md) |
| Find where a struct is populated | [../generators/](../generators/map.md) (writes) / [../handlers/](../handlers/map.md) (reads) |

## Pointers

- Parent: [src/](../map.md).
- Nullable columns are intentional data-quality features (CLAUDE.md §7.4) — keep `Option<...>` fields optional.

## Debug

- A `FromRow` decode error usually means the struct's field type/nullability drifted from `init.sql` — reconcile against the migration.
- Note: there is no `model_status` table here; `cpt_code` / `appointment_cpt` are handled in [../generators/rvu.rs](../generators/map.md), not as standalone model files.

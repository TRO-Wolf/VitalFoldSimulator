# DynamoDB Schema & Write Strategy

VitalFold Engine uses two DynamoDB tables to store patient visit and vitals data, synced from Aurora DSQL. Both tables use on-demand (PAY_PER_REQUEST) billing mode.

---

## Tables

### `patient_visit`

Stores visit metadata: check-in/check-out times, provider seen time, EKG usage, and estimated copay.

| Attribute | DynamoDB Type | Description |
|-----------|--------------|-------------|
| `patient_id` | S (String) | **Partition key** — UUID (stringified) |
| `clinic_id` | S (String) | **Sort key** — composite: `{clinic_bigint}#{patient_visit_uuid}` |
| `provider_id` | S (String) | BIGINT of the attending provider (stringified) |
| `checkin_time` | S (String) | ISO 8601 timestamp |
| `checkout_time` | S (String) | ISO 8601 timestamp |
| `provider_seen_time` | S (String) | ISO 8601 timestamp |
| `ekg_usage` | BOOL | Whether an EKG was performed |
| `estimated_copay` | N (Number) | Decimal dollar amount ($20-$150 standard, $150-$350 EKG) |
| `creation_time` | N (Number) | Unix epoch seconds |
| `record_expiration_epoch` | N (Number) | Unix epoch seconds (creation + 90 days) |

### `patient_vitals`

Stores vital sign measurements taken during the visit.

| Attribute | DynamoDB Type | Description |
|-----------|--------------|-------------|
| `patient_id` | S (String) | **Partition key** — UUID (stringified) |
| `clinic_id` | S (String) | **Sort key** — composite: `{clinic_bigint}#{patient_visit_uuid}` |
| `provider_id` | S (String) | BIGINT of the attending provider (stringified) |
| `visit_id` | S (String) | UUID linking back to `patient_visit` |
| `height` | N (Number) | Inches (60-78) |
| `weight` | N (Number) | Pounds (120-220) |
| `blood_pressure` | S (String) | Format: `"120/80"` |
| `heart_rate` | N (Number) | Beats per minute (50-120) |
| `temperature` | N (Number) | Fahrenheit (97.0-99.5) |
| `oxygen` | N (Number) | SpO2 percentage (95-100) — DynamoDB attribute name is `oxygen` (not `oxygen_saturation`) |
| `creation_time` | N (Number) | Unix epoch seconds |
| `record_expiration_epoch` | N (Number) | Unix epoch seconds (creation + 90 days) |

---

## Key Design

Both tables share the same key structure:

- **Partition key:** `patient_id` (UUID string) — distributes data evenly across partitions
- **Sort key:** `clinic_id` — composite value formatted as `{clinic_bigint}#{patient_visit_uuid}` (e.g., `"5#a1b2c3d4-..."`)

The composite sort key ensures uniqueness: a patient can have multiple visits at the same clinic, each distinguished by the visit UUID suffix. Note that `clinic_id` in Aurora is a `BIGINT` identity column (not UUID) as of the recent schema refactor — the integer value is stringified into the composite key.

---

## Data Flow

```
Aurora DSQL                              DynamoDB
┌─────────────────┐                     ┌──────────────────┐
│ patient_visit    │──── JOIN ───────────│ patient_visit    │
│ patient_vitals   │     on visit_id    │ patient_vitals   │
└─────────────────┘                     └──────────────────┘
         │                                       │
         └── SELECT ... WHERE                    └── PutItem per visit
             checkin_time::date IN range              (2 writes per visit)
```

**Triggered by:**
- `POST /simulate` — syncs today's visits
- `POST /simulate/date-range` — syncs a date range

**Source query:** JOINs `vital_fold.patient_visit` and `vital_fold.patient_vitals` on `patient_visit_id` in Aurora, filtering by `checkin_time::date` within the target range. Each result row produces two DynamoDB `PutItem` calls — one per table.

---

## Write Strategy

### Bounded Concurrency

DynamoDB writes use a **tokio semaphore** capped at 40 in-flight requests. This stays well within on-demand mode's initial throughput of 4,000 WCU per table while preventing connection pool exhaustion.

### Retry with Exponential Backoff

On `ThrottlingException` or `ProvisionedThroughputExceeded`:

| Attempt | Base Delay | Jitter | Max Delay |
|---------|-----------|--------|-----------|
| 1 | 50ms | 0-25ms | 75ms |
| 2 | 100ms | 0-50ms | 150ms |
| 3 | 200ms | 0-100ms | 300ms |
| 4 | 400ms | 0-200ms | 600ms |
| 5 | 800ms | 0-400ms | 1200ms |

After 5 retries, the write is logged as failed and skipped. The simulation run continues — DynamoDB write failures do not abort the operation.

### Idempotency

`PutItem` is idempotent on the same key (`patient_id` + `clinic_id`). Running the same date-range sync twice overwrites with identical data. Safe to retry.

---

## TTL (Time to Live)

Both tables include `record_expiration_epoch` set to `creation_time + 90 days` (Unix epoch).

To enable automatic cleanup, configure DynamoDB TTL on this attribute:

```bash
aws dynamodb update-time-to-live \
  --table-name patient_visit \
  --time-to-live-specification "Enabled=true, AttributeName=record_expiration_epoch"

aws dynamodb update-time-to-live \
  --table-name patient_vitals \
  --time-to-live-specification "Enabled=true, AttributeName=record_expiration_epoch"
```

DynamoDB deletes expired items within 48 hours of the epoch timestamp. TTL deletes do not consume WCU.

---

## Capacity Estimates

**On-demand mode** — no capacity planning required. Approximate costs:

| Operation | RCU/WCU | Notes |
|-----------|---------|-------|
| PutItem (1 visit) | 1 WCU per table | ~1KB item, 2 writes per visit |
| Scan COUNT (db-counts) | 0.5 RCU per 4KB | For 200 items (~100KB): 1 page, ~13 RCU |
| Batch delete (reset) | 1 WCU per item | Paced at 50ms between batches (~500 WCU/s) |

---

## Reset

`POST /simulate/reset-dynamo` deletes all items from both tables:

1. Scans each table to get all keys (partition + sort key only)
2. Deletes in batches of 25 (DynamoDB `BatchWriteItem` limit)
3. Paces 50ms between batches to avoid throttling
4. Retries with exponential backoff on throttle errors
5. Progress reported via `dynamo_progress` field in `GET /simulate/status`

---

## JSON Schema

The canonical JSON schema is in [dynamo.json](dynamo.json).

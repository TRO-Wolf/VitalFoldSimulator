# Page Specifications

## Login Page (`#/login`)

### Layout

```
+--------------------------------------------------+
|  VitalFold Engine                                 |
+--------------------------------------------------+
|                                                    |
|         +----------------------------+             |
|         |   [ Login | Register ]     |   tab bar   |
|         |                            |             |
|         |   Email:    [___________]  |             |
|         |   Password: [___________]  |             |
|         |                            |             |
|         |   [    Sign In / Up    ]   |             |
|         |                            |             |
|         |   (error message here)     |             |
|         |                            |             |
|         |   "Admin? Login here"      |   link      |
|         +----------------------------+             |
|                                                    |
+--------------------------------------------------+
```

### Admin Login Variant

When "Admin? Login here" is clicked, the form changes to:

```
|         |   Username: [___________]  |
|         |   Password: [___________]  |
|         |   [    Admin Login     ]   |
|         |   "Back to user login"     |
```

### Behavior

- **Tab toggle:** Switches between Login and Register. Both use the same form fields (email + password), but submit to different endpoints.
- **Login:** `POST /api/v1/auth/login` with `{ email, password }`
- **Register:** `POST /api/v1/auth/register` with `{ email, password }`
- **Admin Login:** `POST /api/v1/auth/admin-login` with `{ username, password }`
- **On success:** Store `token` in `sessionStorage`, navigate to `#/dashboard`
- **On error:** Display error message below the form (red text, Pico's `[role="alert"]`)
- **If already authenticated:** Redirect to `#/dashboard` immediately

### State

```javascript
const [mode, setMode] = useState('login');       // 'login' | 'register' | 'admin'
const [email, setEmail] = useState('');
const [password, setPassword] = useState('');
const [error, setError] = useState('');
const [loading, setLoading] = useState(false);
```

---

## Dashboard Page (`#/dashboard`)

### Layout

```
+--------------------------------------------------+
|  VitalFold Engine    user@email.com    [Logout]   |
+--------------------------------------------------+
|                                                    |
| +------------------+  +------------------------+  |
| | STATUS           |  | CONTROLS               |  |
| |                  |  |                         |  |
| | [*] Running      |  | [Populate]  [Simulate]  |  |
| |   or             |  | [Stop]                  |  |
| | [o] Idle         |  |                         |  |
| |                  |  | [Reset Aurora]           |  |
| | Last run:        |  | [Reset DynamoDB]         |  |
| | Mar 7, 2026      |  |                         |  |
| | 2:30 PM          |  |                         |  |
| +------------------+  +------------------------+  |
|                                                    |
| +----------------------------------------------+  |
| | POPULATE CONFIGURATION                        |  |
| |                                               |  |
| | Providers     [  50  ]   Patients  [ 50000 ]  |  |
| | Plans/Company [   3  ]   Appts/Pat [     2 ]  |  |
| | Records/Appt  [   1  ]                        |  |
| +----------------------------------------------+  |
|                                                    |
| +-------------------------+ +------------------+  |
| | AURORA DSQL             | | DYNAMODB         |  |
| |                         | |                  |  |
| | Insurance Companies   7 | | Patient Visits 0 |  |
| | Insurance Plans      21 | | Patient Vitals 0 |  |
| | Clinics              10 | +------------------+  |
| | Providers            50 |                        |
| | Patients         50,000 |                        |
| | Emergency Contacts  ... |                        |
| | Demographics        ... |                        |
| | Patient Insurance   ... |                        |
| | Clinic Schedules    ... |                        |
| | Appointments    100,000 |                        |
| | Medical Records     ... |                        |
| +-------------------------+                        |
+--------------------------------------------------+
```

### Sections

#### 1. Status Card
- **Running indicator:** Green pulsing dot + "Running" text, or gray dot + "Idle"
- **Last run:** Formatted timestamp from `last_run` field, or "Never" if null
- Source: `GET /simulate/status` response fields `running` and `last_run`

#### 2. Controls Card
- **Populate button:** Reads values from the Populate Config form, sends `POST /populate` with JSON body
- **Simulate button:** Sends `POST /simulate` (no body)
- **Stop button:** Sends `POST /simulate/stop`. Only enabled when `running === true`
- **Reset Aurora button:** Shows confirm-modal, then sends `POST /simulate/reset`
- **Reset DynamoDB button:** Shows confirm-modal, then sends `POST /simulate/reset-dynamo`
- **When running:** Populate and Simulate buttons are disabled. Stop is enabled.
- **When idle:** Populate and Simulate are enabled. Stop is disabled.
- **Reset buttons:** Always enabled but always require confirmation

#### 3. Populate Configuration
- Input fields with defaults matching the API:
  - `providers` (default: 50)
  - `patients` (default: 50000)
  - `plans_per_company` (default: 3)
  - `appointments_per_patient` (default: 2)
  - `records_per_appointment` (default: 1)
- Values read when Populate button is clicked
- Number inputs with `min="1"` constraint

#### 4. Data Counts
Split into two sub-cards:

**Aurora DSQL** (11 fields):
| Label | JSON field |
|-------|-----------|
| Insurance Companies | `insurance_companies` |
| Insurance Plans | `insurance_plans` |
| Clinics | `clinics` |
| Providers | `providers` |
| Patients | `patients` |
| Emergency Contacts | `emergency_contacts` |
| Demographics | `patient_demographics` |
| Patient Insurance | `patient_insurance` |
| Clinic Schedules | `clinic_schedules` |
| Appointments | `appointments` |
| Medical Records | `medical_records` |

**DynamoDB** (2 fields):
| Label | JSON field |
|-------|-----------|
| Patient Visits | `dynamo_patient_visits` |
| Patient Vitals | `dynamo_patient_vitals` |

All numbers displayed with `toLocaleString()` for comma formatting.

### Polling

```javascript
useEffect(() => {
  const interval = setInterval(() => {
    fetchStatus();
  }, running ? 2000 : 10000);
  return () => clearInterval(interval);
}, [running]);
```

- **When running:** Poll every 2 seconds to show live count updates
- **When idle:** Poll every 10 seconds (or stop entirely — restart on button click)
- **On mount:** Fetch status immediately

### State

```javascript
const [status, setStatus] = useState(null);       // full status response
const [running, setRunning] = useState(false);
const [error, setError] = useState('');
const [confirmAction, setConfirmAction] = useState(null);  // for modal
const [config, setConfig] = useState({
  providers: 50,
  patients: 50000,
  plans_per_company: 3,
  appointments_per_patient: 2,
  records_per_appointment: 1,
});
```

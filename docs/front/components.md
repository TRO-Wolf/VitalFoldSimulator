# Component Specifications

## Nav (`js/components/nav.js`)

Top navigation bar shown on every page.

**Props:**
- `email` — User email to display (null when not logged in)
- `onLogout` — Callback to clear auth state

**Renders:**
```html
<nav class="container-fluid">
  <strong>VitalFold Engine</strong>
  <!-- right side, only when logged in -->
  <span>{email}</span>
  <button class="outline secondary" onclick={onLogout}>Logout</button>
</nav>
```

**Behavior:**
- Logout clears `sessionStorage`, navigates to `#/login`
- On login page: shows title only, no email/logout

---

## StatusBadge (`js/components/status-badge.js`)

Visual indicator of whether the simulator is running.

**Props:**
- `running` — boolean
- `lastRun` — ISO timestamp string or null

**Renders:**
```html
<article>
  <header>Status</header>
  <div>
    <span class="dot {running ? 'dot--running' : 'dot--idle'}"></span>
    <strong>{running ? 'Running' : 'Idle'}</strong>
  </div>
  <small>Last run: {formatted timestamp or 'Never'}</small>
</article>
```

**CSS classes:**
- `.dot` — 12px circle, inline-block
- `.dot--running` — green (#2ecc71), with CSS pulse animation
- `.dot--idle` — gray (#95a5a6)

---

## CountTable (`js/components/count-table.js`)

Displays entity counts in a labeled table format.

**Props:**
- `title` — Section heading ("Aurora DSQL" or "DynamoDB")
- `counts` — Array of `{ label, value }` objects

**Renders:**
```html
<article>
  <header>{title}</header>
  <table>
    <tbody>
      {counts.map(({ label, value }) =>
        <tr>
          <td>{label}</td>
          <td class="count-value">{value.toLocaleString()}</td>
        </tr>
      )}
    </tbody>
  </table>
</article>
```

**CSS:**
- `.count-value` — right-aligned, monospace font for number alignment

**Usage in dashboard.js:**
```javascript
const auroraCounts = [
  { label: 'Insurance Companies', value: status.insurance_companies },
  { label: 'Insurance Plans', value: status.insurance_plans },
  // ... 9 more
];

const dynamoCounts = [
  { label: 'Patient Visits', value: status.dynamo_patient_visits },
  { label: 'Patient Vitals', value: status.dynamo_patient_vitals },
];

html`
  <${CountTable} title="Aurora DSQL" counts=${auroraCounts} />
  <${CountTable} title="DynamoDB" counts=${dynamoCounts} />
`;
```

---

## PopulateForm (`js/components/populate-form.js`)

Input form for configuring population parameters.

**Props:**
- `config` — Object with current values: `{ providers, patients, plans_per_company, appointments_per_patient, records_per_appointment }`
- `onChange` — Callback receiving updated config object
- `disabled` — boolean (true when simulation is running)

**Renders:**
```html
<article>
  <header>Populate Configuration</header>
  <div class="grid">
    <label>Providers
      <input type="number" min="1" value={config.providers}
             onInput={e => onChange({...config, providers: +e.target.value})} />
    </label>
    <label>Patients
      <input type="number" min="1" value={config.patients} ... />
    </label>
  </div>
  <div class="grid">
    <label>Plans / Company
      <input type="number" min="1" value={config.plans_per_company} ... />
    </label>
    <label>Appts / Patient
      <input type="number" min="1" value={config.appointments_per_patient} ... />
    </label>
    <label>Records / Appt
      <input type="number" min="1" value={config.records_per_appointment} ... />
    </label>
  </div>
</article>
```

**Notes:**
- Pico's `<div class="grid">` creates equal-width columns automatically
- All inputs are `type="number"` with `min="1"`
- Values are integers — use `+e.target.value` or `parseInt()`
- When `disabled`, all inputs get `disabled` attribute

---

## ConfirmModal (`js/components/confirm-modal.js`)

Confirmation dialog shown before destructive actions (reset).

**Props:**
- `open` — boolean
- `title` — Dialog title (e.g., "Reset Aurora DSQL Data")
- `message` — Warning text (e.g., "This will permanently delete all Aurora DSQL data. This cannot be undone.")
- `onConfirm` — Callback for confirm button
- `onCancel` — Callback for cancel button

**Renders:**
```html
<dialog open={open}>
  <article>
    <header>{title}</header>
    <p>{message}</p>
    <footer>
      <button class="secondary" onclick={onCancel}>Cancel</button>
      <button class="contrast" onclick={onConfirm}>Confirm Reset</button>
    </footer>
  </article>
</dialog>
```

**CSS:**
- Uses Pico's native `<dialog>` styling
- `.modal-overlay` — semi-transparent backdrop when open
- Confirm button uses `contrast` class (red/danger appearance)

**Usage:**
```javascript
const [confirmAction, setConfirmAction] = useState(null);

// When Reset Aurora is clicked:
setConfirmAction({
  title: 'Reset Aurora DSQL Data',
  message: 'This will permanently delete all generated Aurora DSQL data. This cannot be undone.',
  action: () => api.post('/simulate/reset'),
});

// In render:
html`<${ConfirmModal}
  open=${confirmAction !== null}
  title=${confirmAction?.title}
  message=${confirmAction?.message}
  onConfirm=${async () => {
    await confirmAction.action();
    setConfirmAction(null);
    fetchStatus();
  }}
  onCancel=${() => setConfirmAction(null)}
/>`;
```

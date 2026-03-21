# Implementation Guide

## Prerequisites

- Rust toolchain (cargo)
- Running VitalFold Engine backend (or ability to run it)
- A browser with ES module support (all modern browsers)

## Step 1: Backend — Add Static File Serving

### Cargo.toml

Add to `[dependencies]`:
```toml
actix-files = "0.6"
```

### main.rs

Add import:
```rust
use actix_files;
```

Add to `App::new()` chain, **after** `.configure(routes::configure)`:
```rust
.service(actix_files::Files::new("/", "./static").index_file("index.html"))
```

Order matters — API routes must be registered before the static catch-all, or `/health`, `/api/v1/...` etc. would serve files instead of hitting handlers.

### Verify

```bash
cargo build
# Should compile without errors
```

## Step 2: Create File Structure

```bash
mkdir -p vital-fold-engine/static/css
mkdir -p vital-fold-engine/static/js/pages
mkdir -p vital-fold-engine/static/js/components
```

## Step 3: Build Files (in order)

Build in this order because of import dependencies:

1. `static/index.html` — entry point, loads Pico CSS + app.js
2. `static/css/style.css` — custom styles for status dots, count layout, modal overlay
3. `static/js/api.js` — fetch wrapper (no dependencies on other local files)
4. `static/js/components/nav.js` — no local dependencies
5. `static/js/components/status-badge.js` — no local dependencies
6. `static/js/components/count-table.js` — no local dependencies
7. `static/js/components/populate-form.js` — no local dependencies
8. `static/js/components/confirm-modal.js` — no local dependencies
9. `static/js/pages/login.js` — imports api.js
10. `static/js/pages/dashboard.js` — imports api.js + all components
11. `static/js/app.js` — imports pages + nav, root render

## Step 4: Test Checklist

Run the backend:
```bash
cd vital-fold-engine
cargo run
```

Then open `http://localhost:8787/` in a browser.

### Auth Flow
- [ ] Login page renders at `http://localhost:8787/`
- [ ] Can register a new user — token stored in sessionStorage
- [ ] Can login with existing user — redirects to dashboard
- [ ] Admin login works with env credentials
- [ ] Logout clears sessionStorage and returns to login
- [ ] Opening a new tab requires re-login (sessionStorage is per-tab)

### Dashboard
- [ ] Status card shows "Idle" with gray dot on first load
- [ ] Populate config form shows default values
- [ ] All count values show 0 initially

### Populate
- [ ] Clicking Populate sends POST /populate with config values
- [ ] Status changes to "Running" with green pulsing dot
- [ ] Counts update in real-time during population
- [ ] Populate and Simulate buttons disabled while running
- [ ] Stop button enabled while running

### Simulate
- [ ] Clicking Simulate sends POST /simulate
- [ ] DynamoDB counts update during simulation
- [ ] Stop button stops the simulation

### Reset
- [ ] Clicking Reset Aurora shows confirmation modal
- [ ] Cancel dismisses modal without action
- [ ] Confirm executes reset and refreshes counts
- [ ] Same for Reset DynamoDB
- [ ] Counts return to 0 after reset

### Error Handling
- [ ] Expired token (wait 24h or manipulate) redirects to login
- [ ] Network error shows user-friendly message
- [ ] Double-clicking Populate while running shows appropriate feedback

## API Reference (for frontend integration)

### Status Response Shape

**Important:** The `counts` field uses `#[serde(flatten)]`, so count fields are at the top level:

```json
{
  "running": false,
  "last_run": "2026-03-07T14:30:00Z",
  "insurance_companies": 7,
  "insurance_plans": 21,
  "clinics": 10,
  "providers": 50,
  "patients": 50000,
  "emergency_contacts": 50000,
  "patient_demographics": 50000,
  "patient_insurance": 150000,
  "clinic_schedules": 600,
  "appointments": 100000,
  "medical_records": 100000,
  "dynamo_patient_visits": 0,
  "dynamo_patient_vitals": 0
}
```

### Populate Request Body

All fields optional. Omit any to use server defaults:

```json
{
  "plans_per_company": 3,
  "providers": 50,
  "patients": 50000,
  "appointments_per_patient": 2,
  "records_per_appointment": 1
}
```

### Auth Response Shape

```json
{
  "token": "eyJhbGciOiJIUzI1NiIs...",
  "user": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "email": "user@example.com",
    "created_at": "2026-03-07T10:00:00Z"
  }
}
```

### Error Response Shape

```json
{
  "error": "Description of what went wrong"
}
```

## Maintenance Guide

### File Map (what to edit for common changes)

| I want to... | Edit this file |
|---|---|
| Change login/register form | `js/pages/login.js` |
| Add a dashboard button or section | `js/pages/dashboard.js` |
| Change how API calls work | `js/api.js` |
| Add a new API endpoint call | `js/api.js` (add function), then call from page |
| Change nav bar | `js/components/nav.js` |
| Change status indicator | `js/components/status-badge.js` |
| Change data count display | `js/components/count-table.js` |
| Change populate config fields | `js/components/populate-form.js` |
| Change confirmation dialogs | `js/components/confirm-modal.js` |
| Change colors, layout, spacing | `css/style.css` |
| Change CDN dependencies | `index.html` (Pico CSS), `js/app.js` (Preact/HTM) |

### Adding a New API Endpoint to the Frontend

1. Add a fetch wrapper in `js/api.js` (copy the `get` or `post` pattern)
2. Import and call it from the relevant page component
3. Save and refresh — no build step

### Adding a New Page

1. Create `js/pages/newpage.js` using `login.js` as a template
2. Import it in `js/app.js`
3. Add a hash route (e.g., `#/newpage`) to the route logic in `App()`
4. Add a nav link if needed in `js/components/nav.js`

### How Preact + HTM Works (for backend devs)

HTM is JSX without the build step. You write HTML inside tagged template literals:

```javascript
// This is valid JS — no transpiler needed
const html = htm.bind(h);

function MyComponent({ name }) {
  const [count, setCount] = useState(0);
  return html`
    <div>
      <h1>Hello ${name}</h1>
      <button onclick=${() => setCount(count + 1)}>
        Clicked ${count} times
      </button>
    </div>
  `;
}
```

Key patterns:
- `useState(initial)` — returns `[value, setter]`. Call setter to re-render.
- `useEffect(fn, [deps])` — runs `fn` when `deps` change. Return cleanup function.
- `${condition && html\`...\`}` — conditional rendering
- `${array.map(item => html\`...\`)}` — list rendering
- Props are passed as attributes: `<${Component} prop=${value} />`

### Changing Styles

Edit `css/style.css`. Pico handles most styling via semantic HTML, so you rarely need custom CSS. Use Pico's utility classes (`grid`, `container`, `outline`, `secondary`, `contrast`) before writing custom styles.

### Updating Dependencies

Change the version in the CDN URLs:
- `index.html` line 7: Pico CSS version (`@picocss/pico@2`)
- Every `.js` file: Preact version (`preact@10`) and HTM version (`htm@3`)

To vendor locally (avoid CDN dependency):
```bash
cd vital-fold-engine/static
mkdir vendor
curl -o vendor/preact.mjs https://esm.sh/preact@10
curl -o vendor/preact-hooks.mjs https://esm.sh/preact@10/hooks
curl -o vendor/htm.mjs https://esm.sh/htm@3
```
Then change imports from `https://esm.sh/...` to `/vendor/...`.

### Troubleshooting

| Problem | Cause | Fix |
|---|---|---|
| Blank page | JS error in console | Open browser DevTools (F12) > Console tab |
| Login always fails | Backend not running | Run `cargo run` in `vital-fold-engine/` |
| "Session expired" loop | JWT secret changed or token expired | Clear sessionStorage in DevTools > Application tab |
| Styles look wrong | Pico CSS CDN down | Vendor Pico locally (see above) |
| Buttons in form submit unexpectedly | Missing `type="button"` | Add `type="button"` to non-submit buttons inside `<form>` |
| 404 on page refresh | Hash routing issue | Ensure URLs use `#/` prefix (e.g., `#/dashboard`) |
| CORS errors | Only if frontend served separately | Serve from same Actix server (current setup) |

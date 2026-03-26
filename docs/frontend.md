# Frontend Reference

> Consolidated from: front/architecture.md, front/components.md,
> front/implementation.md, front/pages.md

---

## Architecture

### Stack

| Layer | Choice | Why |
|-------|--------|-----|
| UI framework | **Preact + HTM** (CDN) | React-like hooks/components, 3KB, zero build step |
| CSS | **Pico CSS** (CDN) + custom `style.css` | Semantic HTML = instant professional styling |
| Bundler | **None** | ES modules resolve natively in the browser |
| Serving | **actix-files** from the same Actix server | Single deployment on Render, no CORS |

**No npm, no node_modules, no webpack/vite.** Edit a `.js` file, refresh the browser.
HTM uses tagged template literals that look like JSX but run natively. Preact's
`useState`/`useEffect` hooks handle all state. Pico styles semantic HTML by default.

### CDN Imports

```javascript
import { h, render } from 'https://esm.sh/preact@10';
import { useState, useEffect } from 'https://esm.sh/preact@10/hooks';
import htm from 'https://esm.sh/htm@3';
const html = htm.bind(h);
```

To vendor locally (avoid CDN dependency), download into `static/vendor/` (~15KB)
and change imports to `/vendor/preact.mjs` etc.

### Serving

```rust
// In main.rs App::new(), AFTER .configure(routes::configure)
.service(actix_files::Files::new("/", "./static").index_file("index.html"))
```

API routes registered first — `/api/v1/auth/login` hits the handler, not a static
file. The catch-all `/` serves `index.html` for the SPA.

### Auth Flow

```
Browser                          Actix Server
  |  POST /api/v1/auth/login           |
  |  { email, password }        ------>|  verify bcrypt hash
  |  { token, user }             <-----|  generate JWT (HS256, 24h)
  |  token -> sessionStorage           |
  |  GET /simulate/status              |
  |  Authorization: Bearer <token> --->|  validate JWT
  |  { running, patients, ... }  <-----|
  |  (on 401) clear sessionStorage     |
  |  redirect to #/login               |
```

Token stored in `sessionStorage` — clears when tab closes. The `api.js` wrapper
intercepts 401 responses, clears the token, and redirects to `#/login`.

### Routing

Hash-based, two routes:

| Hash | Page | Auth required |
|------|------|---------------|
| `#/login` (or empty) | Login/Register | No |
| `#/dashboard` | Dashboard | Yes |

Implemented in `app.js` by reading `window.location.hash`. No router library.

---

## File Structure

```
vital-fold-engine/static/
  index.html                  -- Entry point, CDN links, <div id="app">
  css/
    style.css                 -- Custom styles (~285 lines)
  js/
    app.js                    -- Root component, hash router, auth state
    api.js                    -- fetch() wrapper with Bearer token injection
    pages/
      login.js                -- Login / Register / Admin-login form
      dashboard.js            -- Status, controls, counts, polling
      visitors.js             -- Per-clinic visitor list
    components/
      nav.js                  -- Top nav bar (email + logout)
      status-badge.js         -- Running/idle indicator with pulsing dot
      count-table.js          -- Entity count display (Aurora + DynamoDB)
      populate-form.js        -- Population config inputs
      date-range-form.js      -- Date range simulation inputs
      confirm-modal.js        -- Confirmation dialog for destructive actions
      heatmap.js              -- Real-time clinic activity heatmap
```

---

## Components

### Nav (`js/components/nav.js`)

| Prop | Type | Description |
|------|------|-------------|
| `email` | string/null | User email to display |
| `onLogout` | function | Clears sessionStorage, navigates to `#/login` |

Shows title only on login page. Shows email + logout button when authenticated.

### StatusBadge (`js/components/status-badge.js`)

| Prop | Type | Description |
|------|------|-------------|
| `running` | boolean | Running state |
| `lastRun` | string/null | ISO timestamp |

Green pulsing dot when running, gray dot when idle. Shows formatted last run time.

### CountTable (`js/components/count-table.js`)

| Prop | Type | Description |
|------|------|-------------|
| `title` | string | Section heading ("Aurora DSQL" or "DynamoDB") |
| `counts` | array | `[{ label, value }]` objects |

Right-aligned monospace numbers with `toLocaleString()` comma formatting.

### PopulateForm (`js/components/populate-form.js`)

| Prop | Type | Description |
|------|------|-------------|
| `config` | object | `{ providers, patients, plans_per_company, appointments_per_patient, records_per_appointment }` |
| `onChange` | function | Receives updated config object |
| `disabled` | boolean | True when simulation is running |

All `type="number"` inputs with `min="1"`. Uses Pico's `<div class="grid">` for layout.

### DateRangeForm (`js/components/date-range-form.js`)

| Prop | Type | Description |
|------|------|-------------|
| `config` | object | `{ start_date, end_date, appointments_per_patient, records_per_appointment }` |
| `onChange` | function | Receives updated config object |
| `disabled` | boolean | True when simulation is running |
| `onSubmit` | function | Triggers date-range simulation |
| `loading` | boolean | Shows loading state on button |

Native `<input type="date">` calendar pickers. No third-party date library.

### ConfirmModal (`js/components/confirm-modal.js`)

| Prop | Type | Description |
|------|------|-------------|
| `open` | boolean | Visibility |
| `title` | string | Dialog title |
| `message` | string | Warning text |
| `onConfirm` | function | Confirm callback |
| `onCancel` | function | Cancel callback |

Uses Pico's native `<dialog>` styling. Confirm button uses `contrast` class (red).

### Heatmap (`js/components/heatmap.js`)

238-line real-time clinic activity visualization with color scale. Renders a grid
of clinic cards showing appointment activity during timelapse simulation.

---

## Pages

### Login Page (`#/login`)

Two modes: Login and Admin Login.

| Mode | Endpoint | Fields |
|------|----------|--------|
| Login | `POST /api/v1/auth/login` | email, password |
| Admin | `POST /api/v1/auth/admin-login` | username, password |

On success: store token in `sessionStorage`, navigate to `#/dashboard`.
On error: display message below form.

**State:** `mode` (login/admin), `email`, `password`, `error`, `loading`

### Dashboard Page (`#/dashboard`)

Four sections:

**1. Status Card** — Running indicator (green pulsing / gray idle) + last run timestamp.

**2. Controls Card** — Populate, Simulate, Stop, Timelapse, Reset Aurora, Reset DynamoDB.
Populate/Simulate disabled while running. Stop enabled only while running.
Reset buttons always require confirmation modal.

**3. Date Range Form** — Start/end date pickers + appointments/records config.
Defaults to tomorrow. Calls `POST /simulate/date-range`.

**4. Populate Configuration** — providers (50), patients (50000), plans_per_company (3),
appointments_per_patient (2), records_per_appointment (1).

**5. Data Counts** — Two sub-cards:
- Aurora DSQL: 12 fields (insurance companies through patient visits)
- DynamoDB: 1 field (patient visits)

**Polling:**
- When running: every 2 seconds
- When idle: every 10 seconds

### Visitors Page (`#/visitors`)

Per-clinic visitor list showing patient names grouped by clinic. Accessed via
`GET /simulate/visitors`.

---

## API Reference (Frontend Integration)

### Status Response

Count fields are flattened via `#[serde(flatten)]`:

```json
{
  "running": false,
  "last_run": "2026-03-07T14:30:00Z",
  "insurance_companies": 7,
  "insurance_plans": 21,
  "clinics": 10,
  "providers": 50,
  "patients": 50000,
  ...
  "dynamo_patient_visits": 0
}
```

### Populate Request (all fields optional)

```json
{
  "plans_per_company": 3,
  "providers": 50,
  "patients": 50000,
  "appointments_per_patient": 2,
  "records_per_appointment": 1
}
```

### Error Response

```json
{ "error": "Description of what went wrong" }
```

---

## Maintenance Guide

| I want to... | Edit this file |
|---|---|
| Change login/register form | `js/pages/login.js` |
| Add a dashboard button or section | `js/pages/dashboard.js` |
| Change how API calls work | `js/api.js` |
| Change nav bar | `js/components/nav.js` |
| Change status indicator | `js/components/status-badge.js` |
| Change data count display | `js/components/count-table.js` |
| Change populate config fields | `js/components/populate-form.js` |
| Change confirmation dialogs | `js/components/confirm-modal.js` |
| Change colors, layout, spacing | `css/style.css` |
| Change CDN dependencies | `index.html` (Pico CSS), `js/app.js` (Preact/HTM) |

### Adding a New Page

1. Create `js/pages/newpage.js` using `login.js` as a template
2. Import in `js/app.js`
3. Add hash route to `App()`
4. Add nav link in `js/components/nav.js`

### Preact + HTM Patterns

```javascript
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

- `useState(initial)` — returns `[value, setter]`. Call setter to re-render.
- `useEffect(fn, [deps])` — runs `fn` when `deps` change. Return cleanup function.
- `${condition && html\`...\`}` — conditional rendering
- `${array.map(item => html\`...\`)}` — list rendering
- Props as attributes: `<${Component} prop=${value} />`

### Troubleshooting

| Problem | Fix |
|---|---|
| Blank page | Check browser DevTools Console for JS errors |
| Login always fails | Ensure backend is running (`cargo run`) |
| "Session expired" loop | Clear sessionStorage in DevTools > Application |
| Styles wrong | Pico CSS CDN may be down — vendor locally |
| 404 on refresh | Ensure URLs use `#/` prefix |

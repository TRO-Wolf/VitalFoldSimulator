# Frontend Architecture

## Stack

| Layer | Choice | Why |
|-------|--------|-----|
| UI framework | **Preact + HTM** (CDN) | React-like hooks/components, 3KB, zero build step |
| CSS | **Pico CSS** (CDN) + custom `style.css` | Semantic HTML = instant professional styling |
| Bundler | **None** | ES modules resolve natively in the browser |
| Serving | **actix-files** from the same Actix server | Single deployment on Render, no CORS |

## Why This Stack

The goal is a frontend a backend developer can maintain. That means:

- **No npm, no node_modules, no webpack/vite** — edit a `.js` file, refresh the browser
- **No JSX transpilation** — HTM uses tagged template literals that look like JSX but run natively
- **No state library** — Preact's `useState`/`useEffect` hooks handle everything (2 pages, ~5 state values)
- **No CSS framework classes to learn** — Pico styles `<button>`, `<input>`, `<table>` etc. by default

## How It Works

### CDN Imports

Every `.js` file imports Preact and HTM as ES modules:

```javascript
import { h, render } from 'https://esm.sh/preact@10';
import { useState, useEffect } from 'https://esm.sh/preact@10/hooks';
import htm from 'https://esm.sh/htm@3';

const html = htm.bind(h);
```

`html` is then used like JSX:

```javascript
function Hello({ name }) {
  return html`<h1>Hello, ${name}</h1>`;
}
```

### Fallback: Vendor Locally

If CDN reliability is a concern, download the three files into `static/vendor/` (~15KB total) and change imports to:

```javascript
import { h, render } from '/vendor/preact.mjs';
```

## Serving Strategy

The Actix server serves the frontend from `vital-fold-engine/static/`:

```rust
// In main.rs App::new(), AFTER .configure(routes::configure)
.service(actix_files::Files::new("/", "./static").index_file("index.html"))
```

API routes are registered first, so `/api/v1/auth/login` hits the handler, not a static file. The catch-all `/` serves `index.html` for the SPA.

**Dependency:** `actix-files = "0.6"` in Cargo.toml.

## Auth Flow

```
Browser                          Actix Server
  |                                    |
  |  POST /api/v1/auth/login           |
  |  { email, password }        ------>|
  |                                    |  verify bcrypt hash
  |  { token, user }             <-----|  generate JWT (HS256, 24h)
  |                                    |
  |  token -> sessionStorage           |
  |                                    |
  |  GET /simulate/status              |
  |  Authorization: Bearer <token> --->|
  |                                    |  validate JWT
  |  { running, patients, ... }  <-----|
  |                                    |
  |  (on 401) clear sessionStorage     |
  |  redirect to #/login               |
```

**Token storage:** `sessionStorage` — clears when the tab closes. More secure than `localStorage` for a tool that can delete databases.

**401 handling:** The `api.js` wrapper intercepts 401 responses, clears the token, and redirects to `#/login`.

## Routing

Hash-based routing with two routes:

| Hash | Page | Auth required |
|------|------|---------------|
| `#/login` (or empty) | Login/Register | No |
| `#/dashboard` | Dashboard | Yes |

Implemented in `app.js` by reading `window.location.hash` and rendering the matching page component. No router library needed.

## File Structure

```
vital-fold-engine/static/
  index.html                  -- Entry point, CDN links, <div id="app">
  css/
    style.css                 -- Custom styles (~200 lines)
  js/
    app.js                    -- Root component, hash router, auth state
    api.js                    -- fetch() wrapper with Bearer token injection
    pages/
      login.js                -- Login / Register / Admin-login form
      dashboard.js            -- Status, controls, counts, polling
    components/
      nav.js                  -- Top nav bar (email + logout)
      status-badge.js         -- Running/idle indicator with pulsing dot
      count-table.js          -- Entity count display (Aurora + DynamoDB)
      populate-form.js        -- Population config inputs
      confirm-modal.js        -- Confirmation dialog for destructive actions
```

Total: **10 files**. No build artifacts, no generated code.

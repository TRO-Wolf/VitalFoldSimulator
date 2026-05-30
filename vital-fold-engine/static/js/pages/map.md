# `vital-fold-engine/static/js/pages/` — map

**Purpose:** Top-level route views. Each is rendered by `app.js` for one hash route and wires components to API calls.

## Contents

| File | Page | Route |
|---|---|---|
| login.js | Admin / user login form | `#/login` |
| dashboard.js | Counts, populate/simulate/reset controls, progress, heatmap | `#/dashboard` |
| visitors.js | Today's visitors grouped by clinic | `#/visitors` |

## I want to…

| I want to… | Go to |
|---|---|
| Change the dashboard | dashboard.js |
| Change login behavior | login.js + [../api.js](../map.md) |
| Change the visitors view | visitors.js |

## Pointers

- Parent: [js/](../map.md).
- Routing + auth gating live in [../app.js](../map.md); `#/dashboard` and `#/visitors` are protected.

## Debug

- A protected page that bounces to `#/login` means `api.getUser()` returned null — token missing/expired in `sessionStorage`.
- Pages own state + API calls; widgets in [../components/](../components/map.md) are stateless renderers.

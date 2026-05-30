# `vital-fold-engine/static/js/` — map

**Purpose:** The Preact application — bootstrap/router, the JWT-aware API client, and the page/component trees. ESM modules, no bundler.

## Contents

| Entry | What it is |
|---|---|
| app.js | Entry point: hash-router (`#/login`, `#/dashboard`, `#/visitors`), auth gate on protected routes, renders `Nav` + active page |
| api.js | HTTP client; stores token/user in `sessionStorage`, injects `Authorization: Bearer` on every call |
| [pages/](pages/map.md) | Top-level route views |
| [components/](components/map.md) | Reusable UI pieces |

## I want to…

| I want to… | Go to |
|---|---|
| Add a route / page | app.js (router + `PROTECTED_ROUTES`) + [pages/](pages/map.md) |
| Add/Change an API call | api.js |
| Change auth/session storage | api.js (`vf_token` / `vf_user` keys) |
| Build a reusable widget | [components/](components/map.md) |

## Pointers

- Parent: [static/](../map.md).
- Architecture: [../../../docs/frontend.md](../../../docs/frontend.md).

## Debug

- 401 from the API clears nothing automatically — stale `sessionStorage` token can loop you to login; clear it to recover.
- Imports are pinned to `esm.sh/preact@10`; a version bump there can change behavior with no lockfile to catch it.

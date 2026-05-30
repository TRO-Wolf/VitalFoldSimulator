# `vital-fold-engine/static/` — map

**Purpose:** The embedded Preact admin dashboard, served at `/`. **No build step** — Preact 10 + HTM 3 + Pico CSS load via CDN ESM imports. Do not introduce a bundler (CLAUDE.md §7.7).

## Contents

| Entry | What it is |
|---|---|
| index.html | SPA shell: Pico CSS + `/css/style.css`, mounts `#app`, loads `/js/app.js` as a module |
| [css/](css/map.md) | Custom styles layered on Pico |
| [js/](js/map.md) | Preact app: bootstrap, API client, pages, components |

## I want to…

| I want to… | Go to |
|---|---|
| Change app shell / theme / CDN deps | index.html |
| Work on app logic, routing, API calls | [js/](js/map.md) |
| Restyle | [css/style.css](css/map.md) |
| Understand the frontend architecture | [../../docs/frontend.md](../../docs/frontend.md) |

## Pointers

- Parent: [crate root](../map.md).
- Served by `actix-files`; `STATIC_DIR` env var can override the compiled-in path.

## Debug

- A blank page is usually a JS module error — check the browser console; an ESM import 404/typo halts the whole module graph.
- All assets are public (no JWT); the API calls they make are not.

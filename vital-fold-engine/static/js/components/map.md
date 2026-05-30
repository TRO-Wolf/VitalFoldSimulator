# `vital-fold-engine/static/js/components/` — map

**Purpose:** Reusable Preact components composed by the pages. Each exports a single named component.

## Contents

| File | Component |
|---|---|
| nav.js | Top nav bar + logout |
| confirm-modal.js | Confirmation dialog for destructive actions (reset, init-db) |
| count-table.js | Live Aurora/DynamoDB row-count table |
| status-badge.js | Running/idle status indicator |
| date-range-form.js | Start/end date picker for dynamic populate + sync |
| dynamic-populate-form.js | Phase-2 populate controls |
| populate-form.js | Phase-1 / full populate controls |
| populate-calendar.js | Calendar of already-populated dates |
| heatmap.js | Per-clinic hour-by-hour activity heatmap |

## I want to…

| I want to… | Go to |
|---|---|
| Change the populate UI | populate-form.js / dynamic-populate-form.js |
| Change the heatmap render | heatmap.js |
| Reuse the confirm dialog | import `ConfirmModal` from confirm-modal.js |

## Pointers

- Parent: [js/](../map.md).
- Consumed by [../pages/](../pages/map.md); data comes through [../api.js](../map.md).

## Debug

- Components are presentational — if data is wrong, check the page's API call and state, not the component.

# `vital-fold-engine/static/css/` — map

**Purpose:** Custom CSS layered on top of the CDN-loaded Pico CSS base.

## Contents

| File | What it is |
|---|---|
| style.css | Project-specific overrides + layout (heatmap grid, count tables, dashboard chrome) |

## I want to…

| I want to… | Go to |
|---|---|
| Adjust dashboard layout / colors | style.css |
| Change the base framework or theme | [../index.html](../map.md) (Pico CDN link + `data-theme`) |

## Pointers

- Parent: [static/](../map.md).

## Debug

- Pico provides most defaults; if a style "won't take", a Pico rule may be winning on specificity — scope your selector tighter rather than `!important`.

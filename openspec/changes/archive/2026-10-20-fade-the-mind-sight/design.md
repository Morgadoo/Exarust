# Design

## Behaviour

The overlay has a phase, an alpha, a stamp and an end. Idle returns without drawing. Shown sets alpha 1 and, when the end is before the clock, switches to fading out. Fading out subtracts `(now − stamp)/2000`; while the result is at least 0.001 it stays, otherwise the phase becomes idle, alpha 0 and the draw ends. Fading in adds the same amount, clamps at 1 and switches to shown. The stamp is refreshed each drawn tick.

The pick radius is `n · scale · 0.5 · 40 / width`. A candidate at `(cx, cy)` with radius `r` is inside when `√((x−cx)² + (y−cy)²) / r ≤ 1`; the nearest inside candidate wins and a zero radius is skipped.

## Model

`MindPhase`, `MindOverlay { phase, alpha, stamp_ms, end_ms }`, `advance_mind_overlay`, `mind_pick_radius` and `pick_mind_target(candidates, x, y)`.

## Open questions

The billboard basis and the shader upload are separate.

## Docs

`SG_Thaum_Mind` is **6%**. The headline stays 14.4% and 5.7%.

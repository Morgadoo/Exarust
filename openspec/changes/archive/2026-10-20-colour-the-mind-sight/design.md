# Design

## Behaviour

The normal entry has a fixed first colour {0.1, 0.4, 0.6} and second {0.8, 0.6, 0}. Its third is `lerp({0.4, 0, 0}, {0.31, 0.78, 0}, third)`. The two alphas are `0.1 + 0.6·(1 − first)` and `0.1 + 0.6·(1 − second)`, computed in double precision and narrowed. The quad alpha is the overlay's alpha less the entry's fade when the fade is positive.

A kind-1 entry replaces the colours with {0.3, 0.3, 0.3} and {0.5, 0.5, 0.6}, alpha 1 and alpha2 0.5, keeping the same quad alpha rule.

## Model

`MindEntryStyle { fade, kind, first, second, third }` and `mind_entry_colors(style, overlay_alpha) -> MindColors { colour0, colour1, colour2, alpha, alpha2, quad_alpha }`.

## Open questions

The kind-1 third colour and alpha use the pale constants; no third constant was decoded.

## Docs

`SG_Thaum_Mind` is **10%**. The headline stays 14.4% and 5.7%.

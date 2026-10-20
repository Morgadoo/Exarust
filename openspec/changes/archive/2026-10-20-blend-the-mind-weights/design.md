# Design

## Behaviour

The body supplies six values. `A` is a fifth of the first, `B` a fifth of the second plus the third minus the fourth. A marked body (bit `0x800`) or `A` above 1 forces threshold, maximum and ratio to 1. Otherwise `A` is floored at 0 and `B` clamped to 0..1. The maximum is the larger, the ratio `A/(A+B)` (0 when both are 0) and the threshold the largest of `maximum + 0.2`, `0.6·v7c` and `0.015·v74`, capped at 1.

An entry's three values move toward the weights by `k = 2^(−dt/1000)`; a step of at least a second uses `k = 0.5`.

## Model

`MindBodyValues`, `MindWeights`, `MindEntryColors`, `mind_track_weights(body, marked)` and `blend_mind_weights(entry, target, dt_ms)`.

## Open questions

The dwell windows and visibility gating are separate.

## Docs

`SG_Thaum_Mind` is **7%**. The headline stays 14.4% and 5.7%.

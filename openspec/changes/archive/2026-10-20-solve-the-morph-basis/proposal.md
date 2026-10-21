# Proposal

## Why

The morph controller's 767-byte basis solver projects a query point onto a control triangle's plane and returns its barycentric weights. The vertex apply consumes exactly these weights when it computes an offset, so the controller's model is incomplete without it.

## What Changes

- The query moves to the first triangle point, loses its component along the stored normal, and moves back.
- The two weights come from the branch chosen by the absolute x sum against the absolute y sum.
- The third weight is one minus the other two.

## Capabilities

### New Capabilities

### Modified Capabilities

- `morph-controller`: the basis solver.

## Impact

- `exa_assets::morph_controller`, from `morph_barycentric`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `RF_Morphing` becomes **18%** (3,113 of 17,099 bytes). The published headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
- Evidence: the basis solver: the 48-byte control record (the normal, then the three points), the plane projection (the dot in `(x·n + z·n) + y·n` order) with the move back, the x/y/z differences against the third point, the branch on `|x0+x1|` against `|y0+y1|`, both branch formula sets, and `w = 1 − u − v` with the float 1.0. No new corpus files.
- Out of scope: the record builder, which calls this solver.

## Phase

Phase 3, bodies. The morph basis solver.

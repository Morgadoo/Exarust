# Proposal

## Why

The A* chooses between the grid's two floor layers for every step. Its layer branch tree prefers layer 0, falls back to layer 1, marks a "no layer" case for a floor 5–10 above, and refuses a rise outside −20 through +4. Exarust searched a single layer.

## What Changes

- Layer 0 is taken when its floor is set and at most four above the current floor.
- A floor five through ten above marks no layer; more than ten above is refused.
- A set layer 1 wins when higher and within four.
- The chosen rise stays within −20 through +4.

## Capabilities

### New Capabilities

### Modified Capabilities

- `navigation-grid`: the two-layer choice.

## Impact

- `exa_ai::path`, from `choose_path_layer` and `LayerWord`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `TPathFinder` becomes **63%** in `data.json`. The headline stays 17.0%.
- Evidence: the A* search: the two layer words, the `c+4`/`c+10` compares, the `−1` no-layer marker, the layer-1 ceiling rejection and the −20/+4 rise test. No new corpus files.
- Out of scope: the scene-wide countdown.

## Phase

Phase 6, AI. The two-layer choice.

# Proposal

## Why

The point-order adoption (2,748 bytes) adopts a reference mesh's point order on a structurally identical target: it seeds a triangle by matching three corner keys, floods through shared points, and applies the resulting permutation. The caller then adds the part with its target-minus-reference deltas. Exarust has the controller's bake, remap and evaluators but not this adoption.

## What Changes

- Adoption seeds from a target triangle whose three keys match a reference triangle under a rotation, then floods through shared points, matching each unvisited neighbour to an unvisited reference triangle.
- Unmatched points keep their identity.
- Adding a part refuses a point-count mismatch and otherwise records zero weights, target-minus-reference deltas and no children.

## Capabilities

### New Capabilities

### Modified Capabilities

- `morph-controller`: the point-order adoption and part add.

## Impact

- `exa_assets::morph_controller`, from `PointOrder`, `adopt_point_order`, `MorphPartAdd` and `add_morph_part`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `RF_Morphing` becomes **76%** (13,311 of 17,099 bytes). The published headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
- Evidence: the adoption routine: the identity map, the visited arrays, the seed scan with the `{1,2,0}` corner tables, the face incidence, the key comparison in the XY plane with its `1e-8` double, and the final permutation; its caller: the point-count guard, the zeroed part, the node store and the `target − reference` deltas. No new corpus files.
- Out of scope: the node identity pairing and the tag-0x10 stream selection.

## Phase

Phase 3, bodies. The morph point-order adoption.

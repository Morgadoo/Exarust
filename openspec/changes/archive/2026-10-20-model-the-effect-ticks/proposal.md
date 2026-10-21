# Proposal

## Why

Three behaviour groups split out of the original's `RFE_Effects_Electricity` unit are unimplemented: the character's active item effects, the golem's dormancy and homing helpers, and the genuine electric-arc helpers. They carry exact formulas that Exarust can reproduce now.

## What Changes

- The item-effect glow is `0.5·(0.01·level)^0.7`; levels above 0x40 step down by four, lower levels use 0x3c.
- Item charge accumulates by `source·0.004` per tick, clamped to the source.
- A worn part's glow weight is 1 for an odd kind below 0xff.
- Keyed item-effect entries attach once.
- The golem dormancy scales are 0.05 of the source (in) and 0.02 of the glow (out).
- The golem homeward delay is `distance + π + roll − 0.5` and the reach offset is 200.
- The spark colour is `(0, 53, 0)`.
- Live arc parts are kind 1 below 0xfe or kind 0x11 below 0x7f; arc-spark parts are kind 4 below 0xfe.
- The random spark brightness is `0.8 + 0.2·roll`.
- A released electrified weapon effect hides a 0x21-kind node, sets state 1 and clears its callbacks.

## Capabilities

### New Capabilities

- `item-effects`: the character's active item effects.

### Modified Capabilities

- `npc-idle`: the golem's dormancy and homing helpers.
- `rendering`: the electric-arc helpers.

## Impact

- `exa_game::item_effects`, `exa_ai::golem` and `exa_render::electricity`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `Equipment effects and focus-item slot` becomes **20%**, `Golem guardians and alarm zones` **20%**, and `RFE_Effects_Electricity` **10%**.
- Evidence: the four item-effect ticks, the two golem ticks and the five electricity routines, and their constant block. No new corpus files.
- Out of scope: the arc mesh and ribbon buffers, the spark burst, the golem wake/alarm handlers, and the item-effect dispatch and decay.

## Phase

Phase 7, effects. The split effect helpers.

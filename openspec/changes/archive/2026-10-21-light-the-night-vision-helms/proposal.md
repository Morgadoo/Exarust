# Proposal

## Why

The original's player controls own a light of their own: a white glow
over the player's head that turns on while the player wears a helm that
sees in the dark and goes out while a hand holds a torch. Exarust had none,
and did not know which items see in the dark.

## What Changes

- The item body's special kind 1 named (sees in the dark), verified on the
  installed database: exactly the three night-vision helms.
- The item catalogue's kinds carry it (`sight`).
- The switch from what the player wears and holds, read when they change.
- The light: made at 245 cm with intensity 0.01, its ramp to 0.4 by 0.0005
  per millisecond (double precision), 80 cm over the head, kept under the
  ceiling by a 10 cm ball swept up through the level; still while paused.
- `Physics::level_sweep`: a ball swept through the level, not the props.

## Capabilities

### New Capabilities

### Modified Capabilities

- `player-movement`: the player light.
- `apparel`: the special kind that sees in the dark.

## Impact

- `exa_formats::objdb` (`special_kind`, `Burden::SEES_IN_THE_DARK`),
  `exa_items` (`ItemKind::sight`), `exa_physics::Physics::level_sweep`,
  `exa_game::player_light` (new).
- Docs: `docs/PLAYER.md` (The player light), `docs/formats/OTHER.md` (the
  special kind byte). `docs/PLAN.md` is the integrator's.
- Evidence: REA (Ghidra 12.1.4) on the light maker, its frame, the ceiling
  probe, the switch and the equipment's switch, read in the assembly (the
  0.0005 double, 80, 1, 10, the epsilon compare); the switch's byte found
  at data offset 0x214 (the section pointer includes its header) and
  confirmed by a corpus scan of the installed `objdb.rdb`.
- Out of scope: the second rule that puts the light out (a weapon section
  of kind 3 whose held part is lit), the Mind power that forces it on, the
  spectating view's light, equipping a helm in Exarust (no inventory
  window: a normal start keeps the light off, as the original's does).

## Phase

Phase 3, bodies and control: player control matching the original (the
player light, `docs/PLAYER.md`).

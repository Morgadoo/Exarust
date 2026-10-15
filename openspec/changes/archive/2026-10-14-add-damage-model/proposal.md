## Why

Exarust's combat used a stand-in: damage grew with the tip's speed squared,
with no regard for the weapon, where along it the blow lands, or armour.
The original's damage model is the heart of its combat.

## What Changes

- Decode the weapon stat bytes, apparel protection (per-bone coverage,
  padding, slash/crush/pierce, encumbrance, sound class) and races' natural
  armour and health factor.
- Reproduce the factory stat pass (variant bonuses, deform rules by role,
  condition scaling) for assembled weapons.
- Resolve every blow by the original's model:
  - the weapon record, and the zone along the weapon;
  - the speed of the struck point; stabs;
  - the two armour layers per part, with the coverage roll;
  - impact, crush, slash and pierce through padding and resistances;
  - part multipliers;
  - yellow and red health with death; daze; shoves on heavy impact.
- Dressing a fighter builds its armour layers from its worn items and race.
  The held weapon's record and length come from its item and geometry.
- Blade contacts sweep between frames.
- Balance: the assumed stability curve is full within the threshold, so
  attack lunges no longer trip fighters.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `combat`: the original's damage and armour model.

## Impact

- `exa_formats` (`weapon_stats`, `objdb`, `races`, `factory`), the new
  `exa_combat::damage`, `exa_combat`, `exa_character`, `exa_assets::weapon`,
  `exa_world` (worn armour), the viewer, `exa_game::balance`, and
  `docs/formats/OTHER.md`.

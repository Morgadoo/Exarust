# Proposal

## Why

The motile part's holding pass redistributes a held weapon's part tree
with the weapon's table mass, and its length walk, handling element,
add-to-world class bits, impact-counter decay and leaving-weapon blood
release were either missing, unwired or slightly off.

## What Changes

- The equip pass builds a held weapon's part tree from its model's rigid
  nodes and runs the two redistribution passes with the record's table
  mass, then divides the root part's mass word by the holder's size
  (`exa_physics::weapon_mass::WeaponMass`, `exa_character::weapon_mass_of`).
  The result is kept on the weapon holder; no solver stage reads it yet.
- The weapon length walk stops at the first chain node that is not rigid.
- The handling element narrows once from a double subtraction.
- A part joining the world takes its class bits: level contacts on a part
  without foot contacts; a lower leg loses them and its dynamic bit, and
  its children lose the dynamic bit.
- The recent-impact counter decays before the tick's contacts.
- A leaving non-limb part releases its weapon blood spot (tested rule).

## Capabilities

### Modified Capabilities

- `motile-part`: weapon part-tree mass, length walk, world join bits,
  impact counter order and blood release.

## Impact

- `exa_physics` (`weapon_mass`, `part`, `body`), `exa_formats::weapon_record`,
  `exa_combat` (`impact`, `weapon_blood`), `exa_character`, their READMEs,
  `docs/formats/OTHER.md`.
- Evidence: the original's holding pass, the mass redistribution pair, the
  length and sweet-spot walks, the record build's handling element, the
  add-to-world routine, the part step's counter decay and the part's blood
  release, read from the instruction listing.
- Out of scope: held weapons as solver bodies, the shape builder's initial
  part mass (stood in for by the reciprocal of the node's mobility), the
  stored copy of the root mass, the second flag word and collision layers
  the world join writes, wiring the blood release.

## Phase

Phase 4, Combat and items (docs/PLAN.md): the motile part's weapon rules.

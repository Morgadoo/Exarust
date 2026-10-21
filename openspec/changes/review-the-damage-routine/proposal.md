# Proposal

## Why

A review of the damage lane against the original's listing found three
places where Exarust's arithmetic or wiring differs from it. The
conversion that ends a recoil and a follow-through rounds half to even;
Exarust truncated it. A stab whose energy is no number takes the root
branch of the pierce, not the thrust's √2. A turned blow keeps the blow
sag the armour pass set before the turn. Phase 4 (combat and items), the
damage routine (review of `finish-the-damage-routine` and
`block-on-the-shield-arm`).

## What Changes

- `swing::recoil` and `swing::follow_through` round the scaled end
  (scale·400) half to even instead of truncating it.
- `damage::resolve`: a stab's energy that is no number takes the
  √T branch (0 when the shove is nothing), as the listing's unordered
  compare does.
- The hit reaction sends a turned blow's blow sag on a shove with no push
  and no daze.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `combat`: Turned blow; new requirements for the swing end's rounding
  and a stab's pierce without a number.

## Impact

- `exa_combat::swing` (`recoil`, `follow_through`), `exa_combat::damage`
  (`resolve`), `exa_combat` (`react_to_hits`).
- `docs/formats/OTHER.md` (Damage, the striker's recoil),
  `crates/exa_combat/README.md`.

## Evidence

REA (Ghidra) assembly of the damage routine, its recoil and
follow-through helpers (the end conversion is the rounding conversion,
not the truncating one), constants read from the executable.

## Out of scope

Everything the two reviewed changes left out (sounds, script hooks, the
clash counter branch, who raises a guard, weapon bodies).

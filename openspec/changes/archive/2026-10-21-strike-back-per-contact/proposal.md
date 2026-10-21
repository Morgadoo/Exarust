# Proposal

## Why

A second review of the damage lane against the original's listing found
three places where the hit reaction's wiring differs from it. The
original's damage routine runs once per contact, the struck side and the
striker's swing together; Exarust dealt every contact of a frame first
and only then the strikers, so two fighters striking each other in one
frame took their stall and their follow-through in the wrong order. A
clash checks the parrier's state flag 0x400 before the parry. And a
clash or a block marks the swing's weapon and shield sounds once a swing,
the shield's drawing its cooldown from the generator. Phase 4 (combat and
items), the damage routine (review of `finish-the-damage-routine` and
`block-on-the-shield-arm`).

## What Changes

- `react_to_hits` deals each contact's striker right after its struck
  side (`strike_back`), not after the frame's contacts.
- A parrying body that turns blows makes any striker recoil and takes
  nothing else (no lock, no attack-state clear, no sag).
- A clash and a block set the swing's weapon-sound bit (0x1000); a block
  sets its shield-sound bit (0x200) once a swing and draws its cooldown
  from the generator then.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `combat`: new requirements for the turned parry, the striker per
  contact and the guard sounds once a swing.

## Impact

- `exa_combat` (`react_to_hits`, `strike_back`, `StrikerReaction`),
  `exa_combat::swing::flags` (`SHIELD_RUNG`, `WEAPON_RUNG`).
- `docs/formats/OTHER.md` (Damage), `crates/exa_combat/README.md`.

## Evidence

REA (Ghidra) assembly of the damage routine: the clash path's state-flag
check before the parry, the clash's and the block's sound bits and the
block's generator step, the routine called per contact.

## Out of scope

Sound playback itself, the clash's counter branch, who raises a guard,
weapon bodies.

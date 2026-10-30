# Proposal

## Why

The character generator's armour pass (docs/PLAN.md Phase 4, Combat and
items: armour layers from equipped apparel, the `SG_CharacterGen`
component) also measures a head height the part boxes read, and keeps a
NaN coverage share out of its cap. Neither was reproduced.

Evidence: the armour pass's instructions read in the objdump listing and
REA. While walking the body's node of the eleventh slot it compares each
worn piece's display byte (`0x1A7` of the item body) to the running
height with COMISS/JP/JBE, but only when the body's eleventh per-slot
qword (`0x60`) is non-zero; the coverage test comes after and gates only
the layer fold. After the walk it adds (physique − 0.5) × 8 in single
precision. The part-box update reads the result (×0.5 single, ×0.1
double). The coverage cap is COMISS against 1.0 with JP skipping the
clamp, so a NaN share is kept.

## What Changes

- `exa_formats::objdb::Armour`: the display byte and the head mark.
- `exa_formats::encumbrance::head_extra`: the thickest marked display
  plus the physique offset (a tested function).
- `exa_combat::damage::coverage`: the cap is the original's ordered
  compare.
- The review dropped the lane's invented gate on the piece's coverage of
  the eleventh bone (it hid one shipped piece with display 2).

## Out of scope

- A caller for `head_extra`: Exarust's part boxes do not read it yet.
- The armour pass's worn-list purge, the two race/id cases on character
  bit 0x100 and the warmth term over the unidentified object.

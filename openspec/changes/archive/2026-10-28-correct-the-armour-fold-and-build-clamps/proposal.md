# Proposal

## Why

Phase 3 of `docs/PLAN.md` (bodies: the character generator). The chargen4
pass re-read the generator's armour pass, apparel cache, slot walk, body
build, hair refresh and placed spawn against the instructions. Its review
corrected the armour fold's product order, replaced an invented stop of the
slot walk with the original's log-and-continue, and wired the second wind
into every built character.

## What Changes

- `exa_combat::damage::armour_layers`: the effective coverage
  (1 − c)·((c − ½)·1.75) + c in float above ½; the natural layer's coverage
  and padding as double products narrowed once, its resistances float
  products; the fold (1 − s0′)·(s1·γ1) + s0′ with the outer product first.
- `exa_formats::encumbrance::{second_wind, second_wind_of_record}`: a worn
  item of special kind 2 grants the second wind; placed characters and
  scene bodies (the arena's generated fighters among them) set
  `Fighter::second_wind.granted` from it.
- `exa_character::layers`: the garment's layer level reads all twenty
  occupancy words; the slot walk logs "body part stack overflow" past 31
  stack entries and walks on.
- `exa_character::look::{build_feral, build_age}`: the record's feral level
  and age as the build clamps them (0…1, a NaN kept), used by the skin
  darkening and the running look.
- `exa_character::rebuild::hair_rebuild` and
  `spawn::PlacedSpawn::saved_motion_write` as tested rules.

## Evidence

The instructions of the armour pass (layer clear, natural layer, fold,
special-kind switch), the apparel cache's level scan and insert, the slot
assembler's stack push and its message-hook call (level 4: logged and
queued on screen, nothing raised), the body build's three record clamps,
the appearance editor's hair refresh and the placed spawn's saved-motion
reset.

## Out of scope

Special kinds 3, 5 and 7 of the armour pass; the per-bone hit-sound word
and the body-scale write; the dressed entry's relist and texture hand-off;
hair as cloth; a screen calling the rebuild.

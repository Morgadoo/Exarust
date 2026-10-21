# Proposal

## Why

The character extras lane (the AI unit's executor and start-up, the skin
controller, the morph registration, the creation form, the character
physics' shock and lookups, the character databases, the race build and
the motion library) left a branch whose port mostly followed the
decompiler's pseudo-code. A review against the assembly found duplicated
executors, swapped random draws and sound parameters, single-precision
sums where the original works in double, a reversed creation layout, an
invented 'evict the last slot' drop rule, an invented fallback race and
voice numbering read from the wrong template. This change records the
corrected rules.

Phase 3 (Bodies: skeleton, physics, control), the character body and
skeleton milestones; Phase 7 (Game modes and content) for the creation
form and the character databases.

## What Changes

- The attack executor stays `exa_combat::attack::execute`; its main-side
  start sound (pitch from the first draw, gain from the second, in double)
  is added. The AI start-up's `_Temp` node and manager are independent
  checks.
- The electric shock: window, sound, released races, ringing race, burn
  (health floor, shock-stagger bit, motion 3, stagger step, pain) and the
  force release, from the assembly.
- Race voices: the first template's gendered and numbered flags for both
  builders, `{…}` groups dropped, the tens digit unreduced, 15-character
  names, groan sharing in the first builder only; the executable's
  built-in race record for a missing id.
- Character databases: adding stores the object live with an empty entry,
  release by the record's tag, in-place payloads at the same length, save
  without a flush, the original's entry count on load.
- The creation preview frame, the two layouts and the drop routing.
- Morph registration: the target ordinal from the name's second and
  third characters, the rotated control match, the nearest record copy,
  the numbered target deltas; duplicates of existing helpers removed.
- The motion library's fallback and child sort; CPU vertex skinning in
  the original's order; the bone body's flag word.

## Out of scope

- Calling the shock, the start sound, the race voices, the skin helper
  and the morph registration helpers from the running game.
- The creation form's controls, editors, change callback and close.
- The executor's script veto and its motion starters (the body's).
- The per-vertex 32-byte morph records' contents; the old-layout target
  reorder.

## Evidence

REA (Ghidra 12.1.4) on Exanima.exe 0.9.5: the eight units' routines and
their assembly, the matrix, transform, squared-distance and
text-to-integer helpers, the container add and place routines, and the
bytes of the built-in race record and the morph rotation tables. Real
data: the shipped `humanoid.rsq` bodies (`EXANIMA_DIR` / the worker's
install).

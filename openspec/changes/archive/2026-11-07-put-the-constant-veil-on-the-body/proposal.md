## Why

Phase 5 in docs/PLAN.md, component-gap lane thaumgap (round 101): SG_Thaum_DIsp (unit
routines) had Constant Displacement's body-side rules (`thaum::veil`) with no caller, and
TThaumTree's dragged link kept a guessed tangent and differed from the original at its reach.

## Evidence

REA on `Exanima.exe` 0.9.5:

- The constant displacement's creator makes the class, runs its start and adds the shell and
  the tendril image to the scene. The start loads `thmdsp_cns01` (a buffer) and `thmdsp_phs`
  (a set), makes the shell (dead, shared with a later vanish), sets the body's restore hook
  and its hit veto (the phasing callback with the power as its context), sets collision bit
  0x4000 on every physics node of the body, makes the tendril image, and starts the hum
  silent at pitch 1, looping, `(length·0.8)·r` seconds in (double).
- The class's control handler moves the hum to the body and raises it 0.002 while under
  0.12; on its end it adds 0x300 to the hum's flags, releases its reserve, closes the shell
  and ends the tendril image. Nothing takes the veto, the restore hook or the bit away.
- The damage routine calls the veto for a contact of kind 1 (a living limb) after the
  regeneration ramp's reset and the tip check, and returns when it answers yes.
- The phasing callback: streak thresholds `100 >> (s+1)` / `100 − (100 >> (1−s))` against
  the engine generator's percentage; a pass plays `thmdsp_phs` at the body (0.6,
  `0.9 + 0.2·r` in double), ORs 0xC000 into the striker and its descendants, posts the
  tendril at the contact (one strand, budget 50) and lists the striker with the clock.
- The restore hook takes listed strikers with `stamp + 500 < now`, ends the last tendril
  under the striker's owner and clears 0xC000.
- The pair stage skips two nodes whose collision words share a channel bit pair with either
  exclusive (`(a & b & 0x55555550) << 1 & (a | b)`), so the phased striker passes through.
- The shell's "fade hook" is a deferred job on the body's list: the shell's mesh build.
- TThaumTree's link drag: the move handler squares `0.55·zoom` in double and drags the link
  unless the reach is strictly beyond the distance (a NaN drags); the link state keeps the
  angle in radians; the draw takes the engine's sine and cosine of it and its far tangent is
  `(start − pointer)` normalised times `−reach·zoom`; the release reads the stored pointer
  and stores the angle over 2π. The link states are entered only from themselves (the press
  sets 0 or 3; the saved state starts at 0x107), so the drag is unreachable in 0.9.5.

## What Changes

- `exa_combat::veto`: `PhaseVeto` (the streak), `BlowPhased`, the threshold and pitch rules;
  `react_to_hits` asks the veto for limb contacts short of the tip.
- `exa_game::thaum::veil_live`: `LiveVeil`, the displaced bit, the phased blows (sound,
  bits 0xC000, the tendril, the list), the restore hook, and the hum voice.
- `thaum::cast`: the constant cast inserts the veto and the veil; the power's end ends the
  body wisps instead of despawning them. `Wisps::end`.
- `thaum::tree`: the link drag's reach, radians, release point and engine sine/cosine.

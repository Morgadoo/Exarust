## Why

Phase 7 in docs/PLAN.md (thaumaturgy in play), component-gap lane thaumgap, second pass
(round 103): SG_Thaum_DIsp (unit routines) drew its tendrils with a glow stand-in and its
AI Long Door never asked the navigation grid.

## Evidence

REA on `Exanima.exe` 0.9.5:

- The tendril's material is `_TDispMat01`, a 240-byte record the executable builds: emissive
  (0.25, 0.25, 0.25), opacity 0.5, blend word 0x163, shading flags 0x480018, extra 6 (no
  shadows), render state 0x131, one emissive layer `FlameletsA3`. The engine's surface program
  builder makes `shading_f` of it with a colour stream: `vrCol · EmiTex(vrTex) · MatDif.a`.
- The tendril geometry uploads positions, colour bytes, a first coordinate set (side, along)
  and a second (light share, running distance).
- The tendril draw runs once per super-sampling sub-frame with that sub-frame's colour mask
  and composite matrix, its depth term shifted by `((i + first) + ½)·0.002/count − 0.001`
  and restored after.
- The AI's Long Door and the Summon's place pick share one navigation query: the subject
  is the caster's target or the caster; the goal 300 cm back along the subject's forward
  vector; a path of more than one point whose last point lies more than √200 cm away is the
  answer, else the door ends; a busy shared search makes the caller wait.
- The Summon spawns the character record `demon_a01` from the character database and links
  it with familiarity 75 (the literal 0x42960000), topic 0x1c, role 0x21.
- The shell's restore and clear walks set and clear physics-part state bits (0x1000 with a
  1000-step countdown, 0x800, 0x900) used by the physics step.

## What Changes

- `exa_render::force_effects::DispTendrilMaterial` (`disp_tendril.wesl`), drawn by the wisps.
- `exa_game::thaum::long_door_live`: the AI door's grid query, before the casters' walk.
- `LongDoor::nav`; the AI flag from a missing destination.
- `DEMON_RELATION` corrected to 75.
- `exa-tool shader --record <file>`; `--opponent-powers` and `--opponent-turn` debug flags.

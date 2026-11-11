## Why

Phase 7 (Thaumaturgy) in docs/PLAN.md, SG_Thaum_Mind (unit routines), the
raisedead lane (round 175). The feature sweep of round 169 found Raise Dead
and Command Dead never working in the running game ("mind cast 0x10f ...:
code -1"), Spirit Form sometimes not ending in free mode (1 of 5 runs), and
the duellist's Energy Shock leaving an arc voxel light whose ChildOf names
an emitter the simulation does not hold.

## Evidence

REA (Ghidra and the assembly of `Exanima.exe` 0.9.5):

- The Raise Dead cast reads the target's condition word (collapsed 0xA00),
  its rules bit 0x10 and its record's flags straight off the body. On a
  living record kind with enough focus it closes the GUI form whose owner
  is the body, runs the revival when the record's feral level is under
  0.001 and the flags' 0xFF00 byte is clear, then calls the body's revive
  (the same routine the AI manager's rise calls), writes yellow 0.5 and red
  1.0 over the revive's own yellow, spends the cost and plays `ThmMnd_R02`.
- The revival writes the record's feral level 0.2, then (feral above 0) the
  rules bit 8 and the gait scale `1.25 − f²/4`, rebuilds the character from
  the record, and writes the pace, the most energy `1 − f/10`, the stamina
  and the body's copy of the feral level.
- Command Dead's caster unlink (its end) ends the target's marker, calls the
  collapse start on the target and clears a body word beside it.

## What Changes

- `exa_game::mind::runtime`: `MindLive` copies the target's condition,
  rules, feral level and health onto its mind before each bar cast;
  `raise_corpse` (posted to the step's world) runs the revival and the
  revive (`exa_game::rise::revive_corpse`, now shared with the AI manager's
  rise) and writes the raised health; `lay_down` runs Command Dead's
  collapse start; `end_control` resolves the held body and its spirit on
  the frame and posts the end to the step's world; Spirit Form's adoption
  and way home move the power list on the step's world instead of the
  frame's copy, which a slow frame can leave behind the simulation's.
- `exa_game::thaum::live`: an arc emitter's voxel light names its emitter
  (`ArcVoxelOf`) instead of being its child, and a light whose emitter is
  gone is despawned.

## Impact

- Raise Dead raises a corpse that gets up after its knock-out; Command Dead
  raises it and a second cast lays it down again.
- Spirit Form's and Control Mind's end from a cast in the other body no
  longer depends on the frame's copy of the power list surviving the
  bridge.
- Lockstep traces of the arena duel and exanima01 are unchanged.

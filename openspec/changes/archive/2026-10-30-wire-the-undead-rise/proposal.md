# Proposal

## Why

Round 37's motile-part report left the undead rise as missing: death
already stamps the rise clock (`settle_death`, `DeathClock`) and the AI
manager's rule and the revive existed as pure functions, but nothing ran
them, and nothing undid Exarust's go-limp (the clip player, the ticked
clips and the character's controller are removed, and the drive dropped).
The story revive also rebuilt the drive as the human default, which is
wrong for any other race.

Evidence: the original's AI manager frame (its rise branch) and its
revive routine, read in REA (Ghidra listing) and the objdump listing;
the glow fade it calls after the revive (skin material `go`, 1.0).

## What Changes

- `exa_physics::body`: going limp keeps the race's factors and the
  character's stats the muscles read (`Ragdoll::kept_muscles`); a revive's
  drive is built over them (`Drive::of_character`,
  `Ragdoll::revival_drive`), not over `Drive::default()`.
- `exa_game::limp`: go-limp keeps what it removes (`LimpKept`: the clip
  player and the character's controller).
- `exa_game::rise` (new, live): the AI manager's rise rule once a frame
  over non-player dead bodies (`exa_ai::manager::frame`), the revive
  (knock-out, yellow, voice volume = scale × race voice factor, the body's
  muscles over its race and stats, the unarmed set-up through the hands'
  refresh, an AI brain's roles applied again, the death clock dropped),
  the may-rise rule cleared on too little red, and the undoing of go-limp
  for any revived body (clips, controller, heading reset, controller under
  the pelvis), which also serves the story flow's revive.
- `exarust --undead-opponent <frame>`: the debug duellist takes the
  story's body rules and collapses at that frame, to watch it rise.

## Impact

`exa_physics`, `exa_game`, `exarust`; docs/PLAYER.md.

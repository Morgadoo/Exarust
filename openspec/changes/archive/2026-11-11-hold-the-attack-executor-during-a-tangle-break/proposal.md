## Why

The `charai` lane (round 191) closes the gap of two units: the AI glue
(SG_CharacterAI, 19 routines, 4.8 KB) and the motion library
(FLX_Character_Motions, 16 routines, 6.2 KB). Each routine was checked
against REA and `exe.asm`. Almost all of both units was already ported and
live; the backlog's "missing decision routines" and "two large routines
unread" were stale. One live difference was left in the shared attack
executor, and two notes in the code were wrong or missing.

## What the original does

- **The attack executor** (the AI's and the player's): with a held striker,
  after the cast hook, a fighter whose swing does not stall starts nothing
  before its cooldown since the last attack tick, and nothing while its
  motion word is Break Tangle (0x1C). Otherwise it starts the overhead, the
  stab or a side attack as before. The stall branch does not read the
  motion word.
- **The main-side start sound** (a variant of `SwingHA` on the first held
  item's voice at the hip, gain 0.4·v + 0.6, pitch (0.2·u + 0.8) / √scale)
  plays only when the body's quiet byte is clear. The body's set-up (its add
  to the world, run for every character) sets that byte. It is the only
  write to it in the executable, so the sound never plays in 0.9.5.
- **The AI manager frame** also wakes a parked character: a body whose own
  mark is set, under a node flagged as shown, has the mark cleared through
  its node tree. The mark is set only by the body tick's suspension of an
  idle character, which Exarust's body does not keep.
- **The motion part transfer** (start and id-keyed copy) runs when a
  character is added to the world and when the character generator rebuilds
  a body. It carries the shared motion controller's per-part records, and
  Exarust has no such controller.

## What Changes

- `exa_combat::attack`: `Busy::breaking_tangle`; `execute` refuses a free
  start while it is set. The player's and the AI's executor calls set it
  from the clip that plays (`strike_body::BREAK_TANGLE`).
- `exa_combat::attack::main_start_sound`: its doc states that the sound is
  dead in 0.9.5 (the quiet byte is never clear), so it stays unplayed.
- `exa_ai::manager` and `exa_anim::parts`: documented what is not reproduced
  and why.
- Debug aid `EXA_FORCE_TANGLE=<ms>` (one forced tangle break per fighter);
  the player's attack log names the clip the attack replaces, and a held
  request logs `attack held at T ms: Break Tangle plays` at debug level.

## Impact

- Code: `crates/exa_combat/src/attack.rs`, `crates/exa_game/src/player.rs`,
  `crates/exa_ai/src/lib.rs`, `crates/exa_ai/src/manager.rs`,
  `crates/exa_anim/src/parts.rs`, `crates/exa_combat/src/strike_body.rs`;
  READMEs of exa_combat, exa_ai, exa_anim; docs/DEBUGGING.md.
- Behaviour: a fighter clicking or choosing an attack while Break Tangle
  plays no longer cuts the clip short. Nothing else changes; lockstep traces
  of scenes without a tangle break match the base build.

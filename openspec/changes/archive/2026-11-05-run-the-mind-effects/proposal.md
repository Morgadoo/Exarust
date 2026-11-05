# Proposal

## Why

Wiring backlog system S3, the mind-effect runtime. The Mind domain's
casts (`mind::casts`) and its power classes (`thaum::power::mind`) were
pure functions with no game caller: the bar refused every Mind word, no
body carried a mind list or the handover bits, Control Mind never moved
the player, Spirit Form had no spirit body, the combatant's hit made no
mind-power call (`exa_ai::struck` said so), and the brains passed an
empty effect list to the perception.

Phase: the Phase 7 thaumaturgy milestone of docs/PLAN.md (no PLAN.md edit
in this change).

Evidence (REA, Ghidra on `Exanima.exe`, the assembly read for the clock
compares and the zeroed flag bytes): Control Mind's cast, control handler,
handover, the queued camera call, the hand-back's transition callback and
its queued behaviour swap, and the target unlink; Spirit Form's cast, the
spirit-body builder, the control handler, the end, the way home and the
spirit screen's start, stop and tick; the mind-target pick; the combatant
base's take-hit and the mind-break walk it calls; the end-power call; the
transition screen the hand-backs open; the step clear the held behaviour
takes.

## What Changes

- `exa_game::mind::runtime`: the per-body mind component on every
  character, the player's held body, the bar's Mind dispatch at the mind
  pick, the power walk (Control Mind's binding, the possession marker,
  Spirit Form's drain), possession and hand-back, the spirit body's
  adoption and way home behind the transition screen, the hit's break, and
  the effects fed to and taken back from the perception.
- `thaum::cast`: Mind words run the Mind front end; a Use on a body the
  player was handed ends the control.
- `thaum::debug_cast`: a Mind slot aims the pick at the nearest character.
- `exa_ai::brain::perceive_all` reads each brain's `MindPowers`.
- `exarust`: builds the spirit body (race byte 14) when asked.
- The transition form also shows the mind hand-back's alpha.

## Out of scope

- The other Mind classes' ticks (Sense Minds, Mask Mind, Confusion,
  Enrage, Trust, Suspicion, Loyalty, Command Dead): their casts run, their
  entries stay in the list, but nothing ticks them yet.
- The spirit screen's draw and music, the cast sounds (logged only).
- A physical frame for the wraith race: the spirit body stands where it
  is built and does not walk.
- The pick's line-of-sight ray and the rules bit `0x10`.
- Saving a running Control Mind or Spirit Form.

## Why

Lane `ambiguity`. Three times in one week two systems with no order between
them swapped after an unrelated lane added systems, and a test or the game
broke: the arena's end-screen staging against the flow's steps, the
spectate start against the camera's mode records, the cast and vanish
systems. Bevy's ambiguity analysis reports such pairs, but the game only
logged them on request (`--schedule-ambiguities`). The original runs one
fixed order, and lockstep determinism depends on it.

## What changes

- `exarust::schedule_ambiguity_test`: builds the game's App as `main` does,
  headless, in lockstep (`--fixed-frames`), and reads Bevy's ambiguity
  analysis for every schedule of the fixed step (`FixedMainScheduleOrder`'s
  and the ones the simulation bridge moves with it: `SimPreStep`,
  `SimFrameStart`, `CharacterTick`, `BrainChain`) and for `PreUpdate`,
  `Update` and `PostUpdate`. It fails listing each unordered pair and the
  data they conflict on. An explicit allowlist (empty) takes a reason per
  entry; `Update` and `PostUpdate` keep a TODO count of their pairs that
  may only fall. `EXA_AMBIGUITY_DUMP=1` prints each schedule's sorted order,
  edges and pairs.
- Every unordered pair of the fixed step (1149: `FixedPreUpdate` 15,
  `FixedUpdate` 1125, `CharacterTick` 9) and of `PreUpdate` (28) is ordered.
  The fixed step's pairs keep the order the simulation's single-threaded
  executor ran them in before (no pair changed sides); where the original's
  body tick order is known (the health bookkeeping after the attack block,
  the parry check after the swing, the tangle break before the readiness
  work) it agrees.
- `exa_physics::body`: new sets naming the step's phases:
  `MechanismRegistration` (between the behaviour and the character
  controllers), `AfterSolver` (`Observe`, `Sounds`, `Level`, `Strikes`,
  `Sprays`, `Physics`, chained after the solver, before `BodyPostStep`) and
  `MechanismStep` (in `AfterSolver::Physics`); the grips' sync set is placed
  between the character ticks and the solver. `exa_audio` names its two
  fixed-step sets (`ImpactClassSystems`, `BodySoundSystems`).
- `PreUpdate`: the window's own work (the full-screen toggle, the minimize,
  the keys cleared on a focus change) runs after the input is read, before
  the GUI's scale, the pointer's warp and the GUI's focus.

## Evidence

The test passes; on main it listed 1149 pairs in the fixed step's schedules
and 28 in `PreUpdate`. A comparison of each schedule's sorted order before
and after shows no fixed-step pair whose order changed.

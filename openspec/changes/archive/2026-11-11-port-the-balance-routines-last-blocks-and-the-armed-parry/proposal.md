## Why

Phase 3 (the character body) in docs/PLAN.md, TMotileDynamics. The kernel3 lane's block audit of the balance routine
(#18) left eight divergences open, and the armed parry (#25) and the stab's handling pushes (#32), both reached in
0.9.5, were not applied to bodies.

## What the original does

Read in `exe.asm` and the research pack's decompiles:

- **The trigger** clears the out-of-balance bit first and runs for a step of any kind. Its lead tests and normalises
  the whole 3-D fast chest velocity. A body not stepping resets its target to the standing position first; a target
  more than 10·scale cm from it is taken, the episode's landing count reset, and only a body that does not locomote
  sets the stepping (and, from rest, the balance-step) byte. It writes no start clock.
- **The landing pass** holds a reached foot while the body's slow-turn word is set, until 100 ms past the later
  stand-still stamp; the other foot's landing adds 200 ms to that stamp with the word set outside an episode.
- **The scores** measure the reach term from the pelvis eased a quarter of the way toward the stance origin and read
  the stored reach (no crouch factor).
- **The other foot lands** near the target (its guard reads the start clock): only the stepping byte clears.
- **The queued step** is checked on a tick the footwork changed a foot; taken, the chosen foot is not started, and its
  countdown reads the episode's landing count.
- **The armed parry** makes the item's parry link (the soft world weld its attach builds) a world link to the parry
  point with the grip's strength, disables the other hand's, and all of them for a passed blow; its end disables them.
- **The handling pushes** (stab, clip 11) push the item body and the right hand (the body's first hand slot).

## What Changes

- `steps::consider` and `lead`; the start clock (`Steps::started`) left by balance steps; the knockback and collapse
  stagger starts reset the landing count; no start writes the near-target count.
- `Bearing::slow_turn`; the footwork's hold, pelvis reference, uncrouched reach, other-foot clear and queued-step check
  (`steps::QueueView`, `Work::taken`). `crouch_stride` and `foot_step_length` removed.
- `GripLinks::aim_parry`, `release_parry` on the soft weld; `parry::hold_parry_links`; the parry point reads the
  attacker's clock and the passed test.
- `swing.rs drive_stab` applies `weapon::handling_pushes`; `stab::stab_drive` removed.

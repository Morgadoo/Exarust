# Proposal

## Why

Backlog item C8 asked for knocked-out bodies that turn their muscles off
and run the muscles-off sampler, for the knock-out's callers, and for a
caller of the daze/shock pose (`exa_anim::shock_hold`). Tracing the
original showed the premise does not hold:

- The muscles flag goes off only with death (the collapse's dying tick,
  the corpse placement) and comes back with the revive and the player's
  reset. The knock-out writes the condition and the wake nibble, never
  the muscles.
- The body tick returns before any sampling for a dead body. For a body
  whose muscles are off and which is not dead it jumps past the clip
  clock, the clip-end test and both samplers unless the body locomotes
  this tick (its movement decision needs muscles, so it never does) or
  one of two character words is set that no instruction in the
  executable writes (one is only cleared, the other only counted down;
  no data reference to either offset exists). So the muscles-off sampler
  is unreachable in 0.9.5, and Exarust leaving it without a caller is the
  faithful state.
- The knock-out's callers are the new-game, story-start, death-screen and
  load lie-downs (live through the game flow), the revive (live for the
  undead and the story), and the dormant sleeper's sight wake (its sleep
  activity is not installed in Exarust; it belongs with the activities).
- The waking runs right after the behaviour update and before the posture
  block, with no muscles test; Exarust ran it after the posture decision.
- A revived body keeps the balance its dying tick measured (the revive
  does not write it, a dead body's tick never measures it); Exarust's
  revived drive started at balance 1, so its first ten posture decisions
  took the lying, knocked-out body for a standing weak one.
- The player's behaviour releases a held wake (state flag 0x40, set by
  the story's revive) on a mouse button, a movement key or a deflected
  stick; Exarust never cleared it, so a revived story player never got up.
- The daze pose is written by the body tick's daze block every tick the
  shock-stagger flag is set (after the Rebalance start and the energy
  write, before the flag's timer test) and held by the living sampler,
  unless the body's support is under 1, which puts the physical pose back.

Evidence: REA (Ghidra) and the objdump listing of `Exanima.exe` 0.9.5:
the writers of the muscles flag, the body tick's sampler gate and its
locals, the knock-out's callers, the daze block, the waking call site,
the player behaviour's held-flag clear.

## What Changes

- `exa_game::stages::sampler`: a shock-staggered body holds the daze pose
  (`daze_pose` over `exa_anim::shock_hold`: the capture of its Rebalance
  start for the pelvis and legs unless down, a random turn half way from
  the Defence stance's first key for the rest) unless its support is
  under 1. `exa_game::balance::ShockHeld` marks the staggered body.
- `exa_combat::condition`: the knock-out wake runs in the behaviour stage,
  before the posture decision; it logs the start of waking and the wake.
- `exa_physics::body`: going limp keeps the balance; the revival drive
  starts from it.
- `exa_game::player::release_held_wake`: input clears a held wake.
- Docs: the muscles-off sampler is documented as unreachable in the
  original (code docs, `exa_physics` README).

## Impact

`exa_game`, `exa_combat`, `exa_physics`, `exa_anim` (docs).

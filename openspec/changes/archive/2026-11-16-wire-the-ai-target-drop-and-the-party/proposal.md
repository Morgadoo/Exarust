## Why

Roadmap task B4 (lane `motileai3`): TMotileAI's remaining ledger rows. The
combat state still kept a fighter in its guard over a collapsing or dead
target (a `continue` in the duellist), the apply of the roles always passed
the body's rule bit as clear and never told the experience system that a
follower joined or left the party, and the follower arrival never
restarted the idle activity.

## Evidence

REA (Exanima.exe 0.9.5), read in the instructions where order or constants
matter:

- The combat state, after the follow check and the down exit: with a
  target, the focus is cleared, and the target is cleared when its state
  flags carry the vanished bit or its condition carries the collapsing bit
  (collapsing 0x800 and dead 0xA00 both do); with no target and no focus
  the state leaves combat (the leave routine) before the estimate and the
  perception.
- The apply of the roles: the phase word is −1 when the body's rule word
  has bit 4 (the fierce rule), else 0; after the init scripts and the
  mind-control override, a body that followed and no longer does leaves the
  party list, and a body that follows joins it (the join refuses a body
  already listed and starts the record's conversion clock when it is 0).
- The party list holds bodies; the leave swaps the last one into the gap.
- The follower arrival calls the behaviour's restart (searched count and
  busy byte cleared, the idle activity's restart: the search restarts from
  the world origin).
- The room-walk goal callback of the idle driver is referenced only from
  the driver's own states (and its unwind entry); the driver is installed
  only behind a byte nothing writes, so it is unreachable in 0.9.5.

## What Changes

- The duellist drops a collapsing or dead target: the combat tick finds no
  foe and leaves combat after the follow and down exits; nothing perceives
  the dropped target; a dead target's contact still records its death time.
- `brain::PartyChange` (a follower joins or leaves the party) is sent by
  the apply of the roles and the death hook and forwarded by the bridge.
- The apply of the roles reads the body's fierce rule bit for the phase
  word.
- `exa_game::experience_live` keeps the party's bodies with their learners
  (`keep_party`), drops a companion whose body is gone, and writes each
  companion's learner back to its record after a pass.
- `follower_travel::restore_arrivals` restarts an arrival's idle activity
  (`Npc::restart_idle_activity`).

## Impact

- `crates/exa_ai` (lib, brain), `crates/exa_game` (experience_live,
  follower_travel), `crates/exa_formats` (doc of the fierce rule bit).
- Traces change in fights whose target collapses or dies (the fighter
  leaves combat) and for fierce bodies' planned routes (one size smaller).

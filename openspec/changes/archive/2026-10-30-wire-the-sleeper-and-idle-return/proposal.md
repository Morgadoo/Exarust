# Proposal

## Why

Backlog item C4 asked for an activity that installs the go-to-object
state, the idle driver and the investigation, the idle return's chase
caller, and the dormant sleeper's body side with the target pass's wake.
The routines were ported (`exa_ai::lifecycle`, `exa_ai::idle`) but had no
caller in the running game. Phase 6 (NPC behaviour), the TMotileAI
milestone.

## What Changes

- Traced in `Exanima.exe` 0.9.5 (REA, Ghidra, checked against the
  instruction listing):
  - Applying roles maps activity 1, 2, 3, 4, 5 and 7 to the roam, wait,
    sleep, follow, sentry and patrol states; nothing installs the idle
    driver from a role. Its only installer puts it on the player body's
    behaviour, behind a flag byte that nothing writes (zero in the
    uninitialised data). The go-to-object state is installed only by the
    idle driver's object search. So no character runs either in 0.9.5;
    they stay without a caller, as before.
  - The investigation opens the roam's and patrol's two waits and every
    wait tick, but the level pathfinder's disturbance slot (and the
    abstract one's) returns the constant 0, so it never stamps or walks.
    It now runs live with that constant; nothing visible changes.
  - The sleep activity's entry sets the dormant flag, condition 0x600,
    tone and tone target 0, the body's statue byte and gait word 2, and
    installs its tick as both the idle and the current state. The tick
    is not empty, as Exarust had it: every AI tick it ORs the dormant flag
    in and writes condition 0x600 again.
  - The target pass, after weighing, knocks out a body whose flags are
    exactly 0x10 when the best level is above a notice: it removes role 44,
    marks the roles for re-application on the next AI tick, and calls the
    knock-out with no delay.
  - The idle return's callers are the attach, the level-leave reset and
    the chase route's leave (not following, no fresh foe).
  - The perception reads the dormant flag (0x10) as asleep, on the
    observer (reach halved) and on the observed (lying, skipped).
- The one shipped sleeper is `GdnShaperD` (record 0x37 of `exanima67`,
  race Ancient, roles Activity Sleep, 10, Undead; placed down).
- Wired:
  - the sleep entry's body side and the sleep tick in the AI tick, the
    statue byte and gait word kept as the body's statue rest;
  - the target pass's wake: role 44 dropped, the shared knock-out
    (condition down, flags dormant and waking, the wake clock, the body's
    tone, stance and pins) and its wake timer, so the body tick's wake
    raises it;
  - the perception's asleep from the dormant flag;
  - the chase's idle return (look, step 1, current state, mark);
  - the investigation sample with the shipped constant.
- Removed Exarust's stand-in wake from the player rating (a sleeper that
  rated the player above a notice dropped its role without a knock-out).

## Capabilities

### Modified Capabilities

- `npc-idle`: the sleep tick, the sleep and wake in play, the chase's idle
  return, the investigation in play, the unreachable idle driver.
- `npc-perception`: asleep is the dormant flag.

## Impact

- `exa_ai::{brain, lib, lifecycle, idle}`, `exa_game::golem_body`,
  `crates/exa_ai/README.md`, `crates/exa_game/README.md`.
- Evidence: the apply-roles routine, the sleep entry and tick, the AI tick,
  the target pass's wake, the idle return and its three callers, the
  chase route, the investigation and both pathfinders' disturbance slots,
  the idle driver's installer and its flag's references, and the
  perception's flag tests; the shipped records from `exa-tool characters`.
- Out of scope: the idle driver and go-to-object state stay without a
  caller (unreachable in 0.9.5); the AI tick's re-application gate on a
  record word; the idle return's clear of the body's target and focus
  words (the duellist's foe is fixed); the walk-settle byte the idle return
  clears.

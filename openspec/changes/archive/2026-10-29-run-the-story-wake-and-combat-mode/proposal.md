# Proposal

## Why

The round-33 review of the game-main unit left the first wake-up ending on
its first frame (nothing cleared the animator word's 0x20), the combat-mode
setter mapped as the controls, and the story respawn without the controls'
re-attach. The flow also kept its own flags component beside the body's
real state flags.

Phase 7 (Game modes and content), the game-mode flow milestone.

## What Changes

- `flow`: a `CombatMode` step for the player controls' combat-mode setter:
  the practice start (after the controls' attach) and each next fight turn
  it on, the story revive turns it off; the next fight no longer gives the
  controls.
- `flow`: a `KnockOut` step (the lie-down): a new story's unknown
  background lies down to wake 3000 ms on after its first motion; the story
  respawn lies down with no delay before its get-up motion.
- `flow`: a `ReviveBody` step for the dead-body revive the story revive
  makes before it holds the wake and pushes the wake clock 2000 ms on.
- `flow`: the story start and respawn attach the controls again.
- `flow`: the flow reads and writes `exa_combat::StateFlags`; its own flags
  component is gone. The wake pose writes the readiness, the wake clock,
  the stance and the tone.
- `exa_combat::condition::track_wake` runs the wake tick live on a body
  with a knock-out while its flags hold 0x20.
- A camera shot turns the chase camera to its bearing.

## Evidence

The game-main unit's story start, revive, story respawn, practice start and
next fight; the player controls' combat-mode setter, enable/disable and
attach; the body's lie-down, dead-body revive and wake tick, read in the
assembly.

## Out of scope

- The start record's, the level time's and the player-data flags'
  consumers (the save unit's start and write-back do not run live).
- The player behaviour's release of a held wake on input.
- The camera shot's view position, forced speed and cursor warp.

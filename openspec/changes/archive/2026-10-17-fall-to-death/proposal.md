## Why

In Exarust a character that falls into a pit, or off the world, falls
forever: nothing kills it, and a corpse pushed off the world lies far
below for good. The original's body tick checks every living body that is
off its balance and still moving: sinking faster than 5 cm a tick it
screams once; with no ground within 25 m under its pelvis and its feet it
is lost, and 3 s later it dies and leaves the world; the player hears the
pit after 1 s, the screen fades to black over 3 s, and it dies at 4 s. A
kill plane at −50 m takes the dead bodies that fall that far.

Phase 4 (Combat and items: death and injuries), milestone M4 "first
duel", and the master plan's MS5. This is change M44 of the master plan
(`context/re/master-plan.md`, lane F, F8; body note B7), now that the
condition word (M38) and the balance and support (C6) are on main.

## What Changes

- New `exa_combat::fall`: the fall's state (`FallDeath`: stage, the time it
  was lost on the game clock, out of the world), its rules
  (`FallDeath::check`: steady at −5 cm a tick or slower, a scream the first
  tick faster unless dead, lost when the pelvis is 2,500 cm or more over
  the ground under the body and under every foot; the player's pit after
  more than 1,000 ms and death after more than 4,000 ms, anyone else's
  death after more than 3,000 ms; `FallDeath::fall_out`, the kill plane's
  once-only test) and the scream's gain and pitch from two draws
  (`scream_voice`: pitch 0.95–1.05 from the first, gain 0.2–0.35 from the
  second). Every `Fighter` keeps one.
- `condition::PostureBranch`: the branch the posture stage takes, written
  each tick by `track_condition` (the fall check runs in the falls
  branch).
- `exa_physics`: without support the balance measure asks the level for
  the ground under the pelvis (from 20 cm above it, a 20 cm sphere, 20 m
  down, 100 m down on a miss), `Balance::ground`; the get-up's support
  test reads the same query.
- New `exa_game::fall`: the fall check in the posture stage, after the
  condition; the scream (a heavy groan, at the chest, by the character's
  voice volume and pitch), the pit sound `pitdeath01` (gain 0.1, on the
  interface) and the start of the fade; death: dead, yellow and red 0, and
  the character taken out of the world (its root and everything under
  it). After the body solver the kill plane: a dead body whose root fell
  below −50 m loses its health and, unless it is the player's, is taken
  out.
- New `exa_ui::fade`: the fade to black, a black layer over the view and
  under the HUD's bars whose opacity rises to 255 over 3 s of game time.
- `exa_combat::Roll` (the original's generator) is public, for the
  scream's draws.

Evidence:
- `exe.asm` and constants read from `Exanima.exe` 0.9.5: the fall check,
  its two call sites, the pit sound routine, the voice routine and the
  voice sets' slots, the session's player-death hook and the HUD's fade,
  the character ground's query, the kill plane and the character's
  kill-plane callback (`context/re/master-body.md` §9, corrected in the
  new §24).
- The installed corpus: `pitdeath01` and the `vhm01`–`vhm09` `_groanh`
  sets in `Sound.rpk`.
- Light runs of the player dropped from 200 m in the arena (timeline
  ticks and paused shots before and during the fade).

Out of scope:
- the retail check RR18 (the exanima02 chasm), which waits for a capture;
- the pause at the fade's end and a death menu; the world sounds fading
  with it; the camera's tilt nudge (the original's camera overwrites it
  every 17 ms);
- characters' own voice sets (the scream picks a human voice, as the hit
  sounds do) and the voices' hold-off; a possessed body's return and the
  status effects ending (none in Exarust);
- the dead branch's fall check (unreachable in the original: a dead body's
  tick ends before it);
- impact damage from landing (C22).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `character-stats`: falling to death and the kill plane (new
  requirements).
- `physical-body`: the ground under an unsupported body (new requirement).
- `user-interface`: the death fade (new requirement).
- `audio`: the fall's sounds (new requirement).

## Impact

- `exa_combat` (`fall.rs` with `fall_test.rs`, `condition.rs`,
  `fighter.rs`, `lib.rs`)
- `exa_physics` (`body_balance.rs` with its test: lane A's file, a field
  and the unsupported pelvis query)
- `exa_game` (`fall.rs` with `fall_test.rs`, `lib.rs`)
- `exa_ui` (`fade.rs`, `lib.rs`)
- `docs/formats/RSQ.md`, `docs/UI.md`, `docs/PLAYER.md`, `docs/PLAN.md`,
  `docs/DEBUGGING.md`, the READMEs of `exa_combat`, `exa_game`, `exa_ui`
  and `exa_physics`

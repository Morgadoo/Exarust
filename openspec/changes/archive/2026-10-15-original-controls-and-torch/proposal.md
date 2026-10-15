## Why

The player moved relative to the camera and the right mouse button orbited
the camera. The original works the other way round: W walks the
character's own heading, A and D steer it, the right mouse button walks
toward the cursor, and only the middle button orbits the camera, which
never turns by itself. With the original's habits, holding the right button
to walk turned Exarust's camera, and the walking direction turned with it:
the player walked in a circle. The camera also sat 4.5 m behind with a
fixed field of view, where the original frames the dungeon from a fixed
20 m and zooms by field of view. The player carried no torch, and the
original's own torch, held in the left hand with the "1H & Torch" moveset,
was missing.

A blow on a muscle-driven body could also run away: deep contact pushes
became velocity, and a scale picked up from the deforming pelvis grew the
fallen body's targets every tick.

## What Changes

- Tank controls in exploration:
  - W walks the heading; A and D steer 0.025 rad per tick, or turn a
    quarter in place when standing;
  - S brakes, then backpedals;
  - the right mouse button walks to the cursor and runs beyond 3 m;
  - the original's run blend, body turn rate and wall look-ahead;
  - steering runs in the 83 Hz fixed step.
- Combat mode on Tab, with footwork relative to the cursor direction.
- The original's chase camera:
  - a fixed 20 m distance with zoom by field of view (six stops);
  - pitch from the zoom with the orbit drag, on the middle mouse button;
  - the orbit spring at the original's 17 ms camera tick;
  - starting behind the player.
- The player carries the game's torch (`torcha02`) in the left hand, with
  the "1H & Torch" moveset.
- Ragdoll guards: particles move at most 25 m/s, one contact push is at
  most twice the sphere's radius, a fallen body aligns its targets to the
  pelvis's place and turn only, and a target step's scale is shared over
  its ticks.
- `--autowalk` ignores every key but its own.

Not fixed here: a muscle-driven body struck by a blow can still sink
through the floor and drift away (seen with `--shove` in exanima01; the
build before today shows it too). It is the next physics task.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `player-movement`: tank controls, walking to the cursor, combat
  footwork, the original's camera, the carried torch.
- `physical-body`: ragdoll stability.

## Impact

- `exa_game` (`player.rs`, `torch.rs`, `lib.rs`), `exa_physics`
  (`lib.rs`, `body.rs`), `exarust` (`viewer.rs`), `docs/PLAYER.md`,
  `docs/INDEX.md`.

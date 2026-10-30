# Proposal

## Why

The wiring backlog's E2: the door, lever and push-surface steps already
worked out every creak, knock, slam, unlatch, grind and click level, but
nothing played them (`body_door`, `body_lever` and `body_plate` said so).
The door's lock sound was played, but from a single file named
`door?_LkO`, which the packs do not hold: the lock sounds are numbered sets
(`doorb_lko01`), as the door's set-up loads them, so it was silent. The
unlock (the right key), the wrong key and the hand's rattle of a latched
door played nothing either. A report also said a teleport journey's bubble
was stepped twice a tick.

Phase 6 (the living level), the mechanism milestone.

Evidence: REA (Ghidra) reads of the original's hinged and sliding door
steps, the door's set-up (its five set loads and the collision-set
default), its open and unlock, the hand's latched-door press and the wrong
key, the lever step and its return arming, the push-surface step, the
mixer's voice restart and release, and the teleport bubble's own step; the
installed `Sound.rpk` listing (`door?_hng`, `slm`, `opn`, `lko`, `lkt`,
`leverb_mov`, `leverb_tck01`, `stnb_mov`, `stnb_fit`).

## What Changes

- `exa_physics::mechanism_sound` (new): the request message and the pure
  builders that turn a door, lever or plate tick into it (sets, points,
  levels), the lock cue and the rattle's levels; `door::Sound::levels`
  draws the pitch's share first, then the volume's, in double precision.
- The door (hinged and sliding), lever and plate bodies send their ticks'
  requests; a lever armed at load starts its tick loop.
- `exa_audio::mechanisms` (new): one restarted voice per body, the
  one-shots (a hinged door's knock on its own collision set when it has
  one), the lever's looping tick file; every restart and one-shot logged
  with its set, volume and pitch.
- The door's lock sound plays its numbered set.
- `exa_game`: the unlock plays `LkO`, the wrong key and the hand's rattle
  of a latched door play `LkT`.
- `exarust --mechanism-poke <frame>`: a debug flag that pulls the nearest
  door and turns the nearest lever through the live steps.
- The journey bubble's single step is pinned by a test: the plain bubble
  tick already leaves a journey's bubble to its journey.

## Out of scope

The struck item's own clash sound (C1: ten sound calls in the damage
routine to map), the door body's plain collision sounds with the slam set
as its default (the impact class), the navigation passage report, the
lever's linked light, the runtime library's own random stream (the mixer
draws from the engine generator), the second sound environment some door
calls name.

## Impact

`exa_physics`, `exa_audio`, `exa_game`, `exarust`.

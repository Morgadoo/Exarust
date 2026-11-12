## Why

Stage 6, 1a part 3: the last lent types written on both sides. This change takes the gizmo storages, the effects' random streams and the bodies' sounds; the collision world, the remaining sounds and the conversations follow.

## What Changes

- `FrameCommands::send`: work the frame does with a step's results, run after the step behind the bridge and after `FixedLast` in one world (`FrameCommandsStep`).
- Gizmos: each side keeps its own drawing buffer (`bridge_own_resource`); the step's collected gizmos are a snapshot (`bridge_gizmos`).
- Effect random streams: REA shows the original's Mersenne twister (152 call sites) and engine generator (531) are process-wide and drawn from by both threads; the step keeps `EffectDraws`/`EffectRoll` (moved), the frame `FrameEffectDraws`/`FrameEffectRoll`, each in its own order.
- Body sounds: the step rings impacts and runs scrape sources (set sizes from the sound pack, `SoundBank::set_size`); the frame plays and drives the loops by set name (`FrameBodySounds`).

Phase 3 (bodies: skeleton, physics, control) in docs/PLAN.md: the simulation thread.

Rule change: the frame's effects draw from their own streams (before: one stream shared with the step's effects in frame/step order).

## Out of scope

- Physics, StandIns, ColliderNodes, NavGrid; SoundBank, AudioEnvironment, AudioSource (fall, kinesis, shock, throw, alarm sounds); Conversations, ContactMirror; the free loop.

## Why

Roadmap task B4 (lane `simfires`, docs/roadmap/phase-b-close.md): the
particles' body pass is ported and hooked into the solver's object stage
(`ParticleSolve`, round 270), but nothing feeds it: the fires' emitters
step once a frame in the frame's world (`flame::simulate`, its own 83 Hz
accumulator). In the original the particle systems are sector objects:
they step in the simulation and meet the bodies in the solver's object
stage, and the fires burn the characters who stand in them.

## Evidence

REA (Exanima.exe), checked against the assembly:

- The sector step (0x100080cd0) runs every object's step slot (0xd8, the
  particle system's 0x1000f5040) in its object stage, after the bodies'
  bounds pass, in list order, each object whose state & 7 is at most 2,
  stamped with the sector's tick; then body–body detection, then the
  object–body contacts (class bit 0x20000: the particles' body slot).
- The simulation's frame method (0x10008b850) runs on the main thread once
  a rendered frame, unless paused: fraction = 1 + (clock − published
  time)/step, at most 2; then, under the sector-list lock (not the step's),
  every awake object's frame slot (0x110, the particle system's
  0x1000f5c20) with α = (published tick − its stamped tick) + fraction.
  The particle frame routine sets the light from the step's density, walk
  and jitter with its own peak and full-update bit, walks the sound (three
  twister draws), removes a finished emitter's node, and builds the quads;
  the nested builders read α through the parent frame (0x1000f6f00), the
  lerp q + (p − q)·α and the age 1 − (life − α)/L. A just-stepped object
  is drawn at α = fraction, 1 to 2.
- The body-contact class bit is cleared on the fire presets' bed systems
  (0x1000f1ec5, 0x1000f2327, 0x1000f2787) and on the portal marker's
  sparks (0x1000d0b31); the portal's dissolve keeps it.
- A body-attached emitter takes its body's matrix in its step
  (0x1000f5040 → 0x10001c900 from the body's +0x180), at the object stage.

## What changes

- `exa_world::flame`: `Flame` holds the step's state only (the emitter,
  its tick stream, texture, lift, blend, contact bit) and is a snapshot
  behind the bridge; `FlameView` (the frame's, never bridged) holds the
  light's peak and full-update bit, the sound walk, their random stream
  and the quad buffer. `stage_flames` (fixed step, before the solver)
  stages each flame into `ParticleSolve` with its node frame, parent part
  and contact bit; `unstage_flames` (after the solver) takes them back.
  `frame_flames` (frame) does the GI pre-roll (posted with
  `FixedCommands::write`), the removal of a finished flame, the fire's
  tick count and the light and sound walks; `build_meshes` draws at the
  original's fraction (1 + the frame's overstep).
- `exa_world::particle_stage`: `StagedStep`, `step_flame`, `node_frame`;
  `ParticleSolve::sim_objects` ticks the staged flames in the object
  stage, a carried one at its part's pose in the solver.
- `exa_world::particles`: `FrameWalks`, `Emitter::frame_walks`,
  `Emitter::frame_on` (the frame's half, reading the emitter only).
- `exa_world::fire`: the bed is built without the body-contact bit;
  `exa_game::effects::portal`: the marker's sparks too.
- `exa_game::fire_sound` reads the sound walk from `FlameView`.

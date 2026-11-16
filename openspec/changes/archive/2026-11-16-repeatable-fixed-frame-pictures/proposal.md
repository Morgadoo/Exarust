## Why

Roadmap stability lane: the golden suite (`context/tools/golden.sh`) catches
regressions only if two runs of one binary agree, and they did not. With the
traces identical, a scene's pictures still differed from run to run (the
hub's first shot 0.15 of its pixels, the shock scene's floor 0.12, an arena
shot 0.03), and one `--fixed-frames` arena run segfaulted at quit (F567).

## Evidence

Measured on the build worker (RTX 4000, offscreen), each golden scene run
three times on one binary, a frame digest of each frame's picture inputs
compared column by column:

- The frame file's frames were read back asynchronously: a frame reached
  the file one to three frames late, and its loading flag was read when it
  arrived, so the runner's frame numbering (and every shot) moved by a frame
  or two between runs; the runner also caught whatever frame was current
  when it polled (`850:852`).
- Bevy compiled new pipeline variants on the task pool: a body whose skin
  needed a new variant after a Force Blast was left out of the frames until
  the compile ended, a different number of frames each run.
- Background jobs (texture decodes, GI blocks, collider and limb-tree
  builds, dressing, the content stream) and the level build's 25 ms frame
  budget finished at frames that depend on the machine's pace, so the
  loading screen lasted a different number of frames and per-frame state
  that runs behind it (a flame's flicker walk) started play at another
  phase.
- F567: not reproduced in 225 runs of the scene (core dumps on, under gdb,
  under load, the exact crashing binary). At libc's exit only idle pool,
  gamepad, Ctrl-C and stdin threads remain once the device is destroyed;
  a pipeline compile still running on the pool keeps the device and the
  driver's own threads alive into the exit handlers.

## What changes

- A `--fixed-frames` offscreen run reads its frames back in lockstep and
  saves the frames `EXA_OFFSCREEN_SHOTS` asks for itself.
- A `--fixed-frames` run compiles pipelines synchronously, waits for a
  background job when a frame polls it, lifts the load's frame budgets and
  waits for the content stream before its pump.
- `--frame-digest` (debug) hashes each frame's picture inputs.
- The last frame of every run waits for its pipeline compiles and the GPU
  before the renderer is dropped; the exit trace lists the threads alive at
  `main`'s return.

A normal (not `--fixed-frames`) run is unchanged but for the exit wait.

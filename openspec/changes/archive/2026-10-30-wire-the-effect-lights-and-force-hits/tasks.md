# Tasks

- [x] Trace the emissives' registration, update and removal, and the hits' pin gain and landing weight, in REA.
- [x] Scale line and multi-point shapes into the volume's units; 16 points a shape.
- [x] Write the blast's chunk light points in the frame; fix the effect frame's inverse they (and the ribbon) go through.
- [x] Register, refresh and free the blast's, torrent's and teleporter bubble's GI shapes live.
- [x] Write the pin gain and the landing momentum on a blast or torrent hit.
- [x] `--debug-force-cast <s>` debug flag.
- [x] Tests with the original's numbers (charge curve, attenuations, pin gain 0.0768 and recovery, landing weight 5, lifetime).
- [x] GPU run: the shape lights the scene with the GI on, the struck body's pin gain drops and recovers.
- [x] fmt, clippy -D warnings and tests for `exa_render`, `exa_game`, `exa_physics`, `exarust` on the worker.

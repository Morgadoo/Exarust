# Tasks

- [x] `--offscreen <file>`: no winit, schedule runner, window cameras retargeted to one image (interface camera and shadow LOD origin kept), per-frame readback to a shared-memory frame file, stdin input injected as window messages, exit when stdin closes.
- [x] The viewer's `--screenshot` saves the offscreen image.
- [x] `exarust grid`: split-screen cameras with viewports recomputed on resize, a sprite per quadrant fed from the frame file, labels and status, paced launches against `nvidia-smi`, input forwarding to the last clicked quadrant, exit status, Enter and `--restart` restarts, `--screenshot`, `--inject`.
- [x] `exarust grid-swap` and `exarust grid-shot` over the host's Unix socket.
- [x] Unit tests: the input protocol, the frame header, the grid options and injections.
- [x] Desktop runs with exanima01, arenao3, exanima06 and arena: four live quadrants, input reaching only the clicked quadrant, a swap, an ended game's status, frame rates and GPU memory (`context/artifacts/2026-10-30-gridview/`).
- [x] docs/PLAYER.md, crates/exarust/README.md, the usage block in `main.rs`.
- [x] fmt, clippy -D warnings and tests for `exarust` on the worker.

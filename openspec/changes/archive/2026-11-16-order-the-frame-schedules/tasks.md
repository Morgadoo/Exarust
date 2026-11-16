## 1. Phases

- [x] 1.1 `FramePhase` chained in `Update`, `FramePostPhase` chained in `PostUpdate` and placed against Bevy's sets
- [x] 1.2 Every game system of `Update` and `PostUpdate` in its phase

## 2. Pairs

- [x] 2.1 `Update`: each pair inside a phase ordered (5835 pairs to 0, one allowed)
- [x] 2.2 `PostUpdate`: each pair inside a phase and against Bevy's sets ordered (275 pairs to 0, 14 allowed)
- [x] 2.3 The check's TODO counts removed

## 3. Checks

- [x] 3.1 fmt, clippy, workspace tests, play build
- [x] 3.2 Golden suite and determinism on the build worker

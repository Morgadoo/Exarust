## 1. Level build over frames

- [x] 1.1 `exa_world::LevelLoad`: grid sets and rows, terrain tiles and palette materials, level model, props, placed objects, lights, within a time budget per step; `spawn_level` builds the same level at once
- [x] 1.2 Terrain surface maps packed on the async compute pool, at most 24 MiB of textures a step
- [x] 1.3 Static colliders: meshes copied 4 ms a frame, trimesh colliders built on the async compute pool
- [x] 1.4 GI source put in world space and the GI block built on the async compute pool; `GiState` and a refocus around the starting view
- [x] 1.5 GI warm-up 16 steps a frame, its progress reported to the main world

## 2. Characters

- [x] 2.1 A model's mesh groups built on every core; model loading split so garments are fitted off the main thread
- [x] 2.2 Garments painted on the async compute pool, at most 32 MiB of painted textures a frame, 8 ms of dressing a frame
- [x] 2.3 Dressing before the animation systems, so a skinned part is bound in the frame it appears

## 3. Loading screen and viewer

- [x] 3.1 `exa_ui::loading`: the screen, the progress bar, the frame clock and its long-frame log
- [x] 3.2 Level and play modes load behind the screen: level, collision, characters, lighting
- [x] 3.3 `--frames`, `--shove` and `--autowalk` count from the end of the load
- [x] 3.4 Window app id `exarust`

## 4. Verification

- [x] 4.1 Real installation (skipped when `EXANIMA_DIR` is unset): the stepped build of arena, arenao1 and exanima01 equals the build at once (`cargo test -p exa_world -- --ignored`)
- [x] 4.2 Frame times on the RTX 4000: play exanima01 loads in about 15 s with no frame over 190 ms and none over 55 ms in the 10 s after; level exanima01 in about 7 s, longest 153 ms; play arenao1 in about 2.5 s, its frame-2 frame 210–310 ms from the first 4× supersampled frames (130 ms with `--no-supersample`)
- [x] 4.3 Screenshots: the first frame after the load of play exanima01 is lit with the player in braies; `--frames 60` and `--shove 60` act after the load; `--no-gi` ends the load in play and level modes; the character view fits and paints garments
- [x] 4.4 `WAYLAND_DEBUG=1` shows `set_app_id("exarust")`

## 5. Documentation

- [x] 5.1 `docs/UI.md` (loading screen), `docs/RENDERING.md` (GI build and warm-up), `docs/PLAN.md` (Phase 2), crate READMEs
- [x] 5.2 `openspec validate --all --strict`
- [x] 5.3 `make check`

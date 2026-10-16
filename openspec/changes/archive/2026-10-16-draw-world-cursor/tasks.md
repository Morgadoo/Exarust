## 1. Research

- [x] 1.1 REA: the cursor's load routine (size formula, cap 128, resample to 2s × 4s, texture flags 0x133 = linear filters, no mipmaps) and its draw routine (frame offsets per mode, the turn away from the player, the arrowhead's −33° and (0.275 s, 0.4 s), outline alpha (a·(1 − fade))², colours (255, 76, 76, 180) and (255, 205, 76, 255) read from the image, the red's alpha latch, the pin-and-warp option flag)
- [x] 1.2 REA: the GUI scale routine (index 0–5 → 3, 4, 5, 6, 8, 12) and the window-size handler (heights 700, 870, 1051, 1390, 2080; GUISize offset 0); the user's Exanima.ini sets no GUISize and the retail client is 1600 × 1014 (scale 5)
- [x] 1.3 REA: the shapes from the combat branch of the player behaviour tick and the exploration movement request, and its reach test; findings appended to `context/re/corpses-and-markers.md` (§3.6)
- [x] 1.4 Corpus: `exa-tool png Resource cursors.rfi` (256 × 512 A8): stored rows top first arrowhead + crosshair, two chevrons + cross, one chevron, ring; two empty frames
- [x] 1.5 Retail, read-only: a screenshot of the amber arrowhead beside the player between fights; the ring and the arrowhead measured on it and on the earlier combat captures (rim pixels 205 and 160)

## 2. Interface (`exa_ui`)

- [x] 2.1 `scale`: `gui_scale(height, size)` and the `GuiScale` resource following the window's physical height
- [x] 2.2 `cursor`: `CursorShape` with its frames and turn rule, `WorldCursor`, `ScriptedPointer`, `cursor_size`, `colour` with the opaque-red latch, `outline_alpha`, `placement`, `CursorAtlas` (cut, turned upright, area-resampled), the gamma-space opacities (`fill_opacity`, `outline_opacity`)
- [x] 2.3 Five image nodes (four outline copies, the sprite) placed in physical pixels and turned by `UiTransform`; frames rebuilt when the size changes; hidden while loading, without a player view or without a pointer; the system pointer hidden exactly while drawn, never moved; the size logged when built
- [x] 2.4 Tests: sizes 18/24/30/72/128 for scales 3/4/5/12/30; colours, latch and outline alphas 127/255; the frames' places and the turned shapes; the arrowhead's −33°, shift (8, 12) at 30 px and its tip within a pixel of the pointer; chevrons turned away from the player; cutting and turning a synthetic atlas; the area resample; the blend remaps (red 180 over black, outline over floors 30/100/200); GUI scales for heights 480–2160 and the size offset
- [x] 2.5 Corpus test (skipped without `EXANIMA_DIR`): the installed atlas is 256 × 512, 128-pixel frames, the six shapes inked and two frames empty, the arrowhead and chevrons with their tips at the top once turned upright, the ring round

## 3. Game (`exa_game`, `exarust`)

- [x] 3.1 `cursor`: `shape(CursorState)` and `within_reach`; `pick_cursor` reads the player (dead, down, combat mode, the right button with the cursor point's distance, Ctrl, the pointer's ray at the level) and writes `WorldCursor` before the draw; shape changes logged at debug level
- [x] 3.2 `player`: the cursor point on the 80 cm aim plane (`AIM_HEIGHT`; the look-ahead keeps its 90 cm); window positions converted through the scene camera's viewport (`window_ray`, `window_point`), so walking to the cursor and combat facing use the pointer's true point under supersampling; `cursor_ground` uses the scripted pointer
- [x] 3.3 Tests: the combat shapes (ring standing, down and collapsing, arrowhead dead and with Ctrl), chevrons up to 3.0 m and doubled at 3.01 m, not while down, the arrowhead or cross outside combat by reach, reach 3 m across and up, the aim plane 0.8 m
- [x] 3.4 `exarust`: `--cursor x,y`; `--press` with `LMB`, `RMB`, `Ctrl` and `key@a-b` holds; tests of the parser (taps, holds, buttons, bad ranges)

## 4. Verification

- [x] 4.1 In game through `gpu_run.sh --gpu 4000`, `play arena` in the default 1600 × 900 window (GUI scale 5, a 30 px cursor logged), no `Validation Error` or panic in any log:
  - `--opponent --press Tab@100 --cursor 808,452`: the red ring by the player's feet, 30 px, as in reference shot 2 and the retail capture (rim about 128 over the duellist's dark jerkin, retail 160 over the floor)
  - `--opponent --cursor 830,480`: the amber arrowhead tilted with its tip on the pointer, its brightest rim pixel 202 (retail 205 on the screenshot of the same shape)
  - `--cursor 1150,250`: the amber cross over ground beyond reach
  - `--press RMB@60-400 --cursor 800,330` and `--cursor 800,120`: one chevron, then two while running, pointing up the screen, away from the player
  - `--opponent --press Tab@50,Ctrl@100-300 --cursor 860,430`: the red arrowhead with Ctrl held
  - `--opponent --press Tab@50 --cursor 860,430 --frames 3000`: the duellist kills the player 8.7 s after combat mode starts, and the red arrowhead lies beside the corpse, as in reference shot 7
- [x] 4.2 Remove the probes from the tracked tree

## 5. Documentation

- [x] 5.1 `docs/UI.md` (Interface scale, World cursor), `docs/PLAYER.md` (the aim plane, the viewport, the cursor in combat mode, scripted inputs), `docs/DEBUGGING.md` (flags and examples), `docs/PLAN.md` (Phase 4), `docs/COMPONENT-MAP.md` (UI prose), the `exa_ui` and `exa_game` READMEs
- [x] 5.2 `openspec validate --all --strict`
- [x] 5.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR` (292 passed, 3 ignored)

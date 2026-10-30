# Proposal

## Why

Visual checks often need several scenes side by side (a story level, an
arena, a terrain level), and four separate windows were unwieldy. The
user asked for one Exarust window whose render is split into four
1280 × 720 quadrants, each a separate game running its own scene, in the
way Bevy's split-screen example splits a window between cameras. No
separate OS windows and no nested compositor.

An Exarust world holds one game: the current level, the player, the
room visibility, the GI, the audio and some 330 other resources are
world resources, and the original runs one world too. Four games in one
world would mean making all of that multi-instance, so each quadrant's
game runs in a process of its own and the window only composes them.

This is Exarust's own test tooling, not a feature of the original. It
belongs to no `docs/PLAN.md` milestone. Evidence: Bevy 0.20's sources
(the `split_screen` and `many_cameras_lights` examples, `Viewport`,
`gpu_readback`, the headless renderer example) and runs on the desktop.

## What Changes

- `exarust grid [options] <scene1> [scene2] [scene3] [scene4]` (new
  `crates/exarust/src/grid.rs`): one decorated window, four `Camera2d`s
  whose viewports are a quarter of the window's physical size each,
  recomputed on every resize. Each camera draws its own render layer: a
  sprite holding its game's newest frame, under a label and a status line
  aimed at that camera. It launches one child `exarust` a quadrant,
  `play <scene>` by default or a scene's own words when it holds spaces,
  one at a time and only when `nvidia-smi` reports enough free GPU memory
  (4600 MiB a game by default, less what the games still loading already
  hold). It forwards the keyboard, the mouse buttons, the wheel, the mouse
  motion and the cursor to the quadrant last clicked. It shows an ended
  game's exit status and restarts it on Enter or, with `--restart`, by
  itself. `--screenshot` saves the composed window, `--inject` scripts
  host input.
- `exarust grid-swap <n> <scene>` and `exarust grid-shot <file.png>`
  talk to a running host over a Unix socket: replace one quadrant's
  game, save the composed window.
- `--offscreen <file>` (new `crates/exarust/src/offscreen.rs`): the game
  runs without winit, under Bevy's schedule runner, with a primary
  `Window` entity that has no OS window. Every camera that would draw to
  the window draws into one image instead, the interface camera with
  them. The image is read back every frame and written to a
  shared-memory file. Input arrives on standard input and is injected as
  the window's own messages. Standard input closing ends the game.
- The viewer's `--screenshot` saves that image when offscreen.

## Out of scope

- Several games in one world (see above).
- Muting the quadrants that are not active: each game plays its own
  audio.
- Child games re-rendering at a resized quadrant's size: they keep the
  size they started with and the quad scales.

## Impact

`exarust` only. The `play`, `level` and other views are unchanged unless
`--offscreen` is given. docs/PLAYER.md, crates/exarust/README.md and the
usage block in `main.rs`.

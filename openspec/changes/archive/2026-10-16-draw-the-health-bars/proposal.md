## Why

Every frame of the original's fights shows two framed bars at the bottom
centre of the view: health (yellow, then brown for damage that will heal,
then red for lasting damage) and a blue bar beside it. They fade in when
combat mode starts and out after it ends, and a dark red border pulses round
the view when the player's health runs low in story mode. Exarust had no
HUD at all, so every combat comparison with the original's screenshots
lacked them.

Phase 4 (Combat and items), milestone M4 "first duel": the minimal
interface's health display. It is item L14 of the combat-look plan.

## What Changes

- The health bar and the focus bar (the blue bar is the original's focus,
  the thaumaturgy pool), painted as the original draws them: two bars
  48 × GUI scale wide, 52 × scale apart, 5 × scale above the bottom
  (x 550–789 and 810–1049, rows 991–1004 in the user's 1600 × 1014 retail
  window), each a three-segment fill of two-part vertical gradients from the
  original's colour table, the pixel where the first segment ends mixed by
  its coverage, an inner shadow 240 at the edge fading over gs pixels, a
  two-line frame tinted by the stone texture `guitex.rfi` sampled one texel
  per pixel, and a drop shadow 220 × (1 − d/2gs)⁴ outside it.
- The original's fade: fills at the fade squared, frames and shadows at the
  fade; shown while in combat mode, in a frame where the yellow health
  changes or the focus drops: 500 ms in, then a 200 ms hold and 2 s out.
- The block-stamina gauge above the bars with a shield in combat mode
  (drawn by the same painter; Exarust has no shields yet, so it stays
  hidden).
- The heartbeat: a blow that leaves the player's yellow health between
  0.01 and 0.05 starts, at most once a minute and only in story levels, a
  7 s pulse of a dark red border (80, 0, 0) up to alpha 96, every 800 ms,
  reaching a fifth of the window's height in, with the `heartbeat` sound
  looping at gain 0.3.
- Gamma-space compositing: the HUD's layers are composited on the CPU as
  the original blends them (sRGB values), and each pixel's opacity is
  remapped so that Bevy's linear blend matches over the arena's dark floor.
- The interface camera's texture laid over the game's view premultiplied:
  Bevy's default for a later camera (straight alpha) multiplied every
  translucent interface pixel by its opacity twice (half-transparent white
  showed 87 of 255 over the floor instead of about 150); this also corrects
  the world cursor's translucent red and the text's edges.
- Debug helpers: `--health yellow,red` sets the player's starting health,
  `--heartbeat` lets the heartbeat play in any level.
- Docs: UI.md (Health bars, Heartbeat, Compositing), DEBUGGING.md, PLAN.md (Phase 4),
  the exa_ui and exa_game READMEs, COMPONENT-MAP.md.

Evidence:
- Reverse engineering of `Exanima.exe` 0.9.5 (REA, Ghidra, the
  disassembly): the HUD draw routine (placement, order, fades), the
  three-segment bar routines and their colour table, the gradient quad,
  inner shadow, drop shadow (the "Scale" shader's source) and frame
  routines, the HUD skin's palette builder and scheme, the texture mode's
  shaders, the fade routines and their triggers, the character fields
  (yellow interpolated for drawing, red, max, focus and its capacity, block
  stamina, combat mode, shield), the focus regeneration, the heartbeat's
  trigger, cooldown, game-mode switches, draw and sound
  (`context/re/hud.md`).
- Corpus: `guitex.rfi` (512 × 512, mean (111, 113, 117)); the `heartbeat`
  sound (48 kHz mono, 4.83 s).
- Retail screenshots of the user's fights (`r1.jpg`, `r1-hud-bars.png`):
  the bars' positions, segment ends and fill colours.
- Exarust in game through `gpu_run.sh`, with screenshots before and after
  hits and through the fades.

Out of scope:
- focus itself (powers), shields and block stamina: the focus bar stays
  full, as the original's for a character who casts nothing, and the gauge
  hidden;
- the HUD's other parts (power quick-bar, compass, status marker, extra
  indicators), the fade triggers from casting, healing, quick-bar powers
  and interacting with one's own body, and the heartbeat's 1 s fade on
  menus and level changes;
- "second wind" (an item special that replaces the heartbeat);
- game modes: Exarust treats every level but the arenas as story mode.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `user-interface`: the health and focus bars, their fade, the stamina
  gauge, the heartbeat border, the interface's compositing over the view.
- `world-viewer`: the `--health` and `--heartbeat` debug flags.

## Impact

- `exa_ui` (`blend.rs`, `hud.rs`, `heartbeat.rs`, `loading.rs`: the
  interface camera's output, `lib.rs`)
- `exa_game` (`hud.rs`, `lib.rs`)
- `exarust` (`main.rs`, `viewer.rs`: the flags, the heartbeat rule per
  level)
- `docs/UI.md`, `docs/DEBUGGING.md`, `docs/PLAN.md`,
  `docs/COMPONENT-MAP.md`, `crates/exa_ui/README.md`,
  `crates/exa_game/README.md`

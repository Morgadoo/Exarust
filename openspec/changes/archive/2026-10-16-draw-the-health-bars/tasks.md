## 1. Research

- [x] 1.1 REA: the HUD draw routine (placement W/2 − 50·gs, +52·gs, H − 5·gs, 48·gs wide; the gauge at W/2 − 24·gs, H − 9·gs, 2·gs tall; draw order; fade gates), the three-segment bar routines (fill rows y + 2 .. y + 4gs − 4, the mixed pixel as a one-pixel line, the focus bar's guard) and their colour table, the gradient quad (two halves), the inner shadow (240, gs), the drop shadow ("Scale" shader: (1 − r)⁴ over 2·gs, 220) and the frame (two rings, HUD skin palette (129, 134, 135) and (28, 30, 30) at 239, texture mode with `guitex.rfi` one texel per pixel)
- [x] 1.2 REA: the fields the HUD reads (yellow interpolated for drawing, red, max; focus and its capacity from the focus tick; block stamina; combat mode; shield), the fade routines (500 ms in, 200 ms hold, 2 s out; triggers) and the heartbeat (trigger below 0.05 on a non-lethal blow, 60 s cooldown, never in practice or the arena, 7 s, 800 ms pulse to 96, 80/0/0 border to 0.2·H, sound "heartbeat" looping at 0.3); findings in `context/re/hud.md`
- [x] 1.3 Corpus: `guitex.rfi` (512 × 512 BC1, mean (111, 113, 117)); `heartbeat` in Sound.rpk (48 kHz mono, 4.83 s)
- [x] 1.4 Retail screenshots (`r1.jpg`, `r1-hud-bars.png`): bars at x 550–789 and 810–1049, rows 991–1004; frame lines at 548/549 and 790/791; segment ends against the live read (yellow 0.176, red 0.226)

## 2. Interface (`exa_ui`)

- [x] 2.1 `blend`: `Canvas` (straight-alpha layers in gamma space), `linear_opacity` (least-squares fit exact over the floor at 40), `to_srgba8`
- [x] 2.2 `hud`: `HudBars`, `layout`, `Fade`, `FrameTexture`, `paint` (frames, drop shadows, three-segment bars with the mixed pixel, inner shadows, the gauge with its frame on top), one image node in physical pixels repainted on change; hidden while loading or without a player
- [x] 2.3 `heartbeat`: `Heartbeat`, `pulse`, `inset`, `coverage`, `border_image` (an eighth of the window, linearly stretched), the node
- [x] 2.4 `loading`: the interface camera's output laid over the view premultiplied (in-game test pattern: half-transparent white over a grey node 152, predicted 152; over the floor 87 before, about 150 after)
- [x] 2.5 Tests: layout at 1600 × 1014 (550, 810, 989, 240, 16; gauge 680, 969, 10; canvas 538, 968, 524 × 46), scale 4 and 900-pixel windows; the fade (0.5 at 250 ms, 1 at 500; hold to 200 ms, 0.5 at 1200, 0 at 2200; partial rises and falls; a single frame shows nothing); yellow/brown/red at the retail values (mixed pixel 718, brown to 766, red from 767); full bars; focus spent and reserved; dead; frame lines, drop shadow 220·0.95⁴ and corners, the inner shadow at 240·0.7; the fade squared on fills; the gauge; the stone sampled from the bottom; the remap (exact over the floor, black within 3.5 levels over 10–60); compositing order; the pulse (0, 96 at 400 ms, 24 at 200/600/7000 ms, none after 7000), the inset (180 at 900, 203 at 1014), the border image
- [x] 2.6 Corpus test (skipped without `EXANIMA_DIR`): the installed `guitex.rfi` is 512 square with mean (111.5, 112.7, 117.4) ± 2

## 3. Game (`exa_game`, `exarust`)

- [x] 3.1 `hud`: `pick_hud` (the player's health and combat mode; focus 1, no shield), `heartbeat` (start on a blow leaving yellow in [0.01, 0.05) at most once a minute when allowed, the sound looping at 0.3, both ended after 7 s), `HeartbeatRule`, `heartbeat_allowed` (every level but the arenas); health changes logged at debug level
- [x] 3.2 Tests: the trigger (0.08 → 0.04 starts; not within 60 s; not at 0.05, 0.0, 0.009 or without a drop; 0.01 starts) and the levels (exanima01 yes, arena, arenae, ArenaHub no)
- [x] 3.3 `exarust`: `--health yellow,red`, `--heartbeat`, the heartbeat rule per played level

## 4. Verification

- [x] 4.1 In game through `gpu_run.sh --gpu 4000`, `play arena`, 1600 × 900 window (GUI scale 5), no `Validation Error` or panic in any log:
  - `--at 0,0,1 --health 0.176,0.226 --press Tab@50`: the bars against `r1-hud-bars.png` column by column (x 700 yellow: 75 107 129 147 147 133 116 120 130 143 140 120 94 66 against the retail 79 108 131 147 144 129 116 119 128 143 131 121 99 65; x 780 red within 3; x 900 blue within 5 but the middle row; frame inner line 14 against 14, outer 50 against 62)
  - `--press Tab@50` alone: full yellow and blue; `--opponent --press Tab@50`, frames 320, 620, 900: after hits yellow 0.085 / red 0.137 (yellow to 631, brown to 681, red beyond), then dead (brown to the red share, red beyond)
  - `--at 0,0,1 --press Tab@50,Tab@300`: fade 0.096 at 51 ms, 1.0 at 518 ms; after combat mode ends 1.0 held, 0.502 at 1224 ms; no bars when exploring without a blow
  - `--heartbeat --opponent --health 0.06,0.25 --press Tab@5`: the first blow leaves yellow 0.021, the heartbeat starts and the dark red border pulses (41 of 255 at the edge over the black void); in the arena without `--heartbeat`, blows leaving 0.048 and 0.017 start none
- [x] 4.2 Remove the probes from the tracked tree

## 5. Documentation

- [x] 5.1 `docs/UI.md` (Health bars, Heartbeat, Compositing), `docs/DEBUGGING.md` (flags, logs, examples), `docs/PLAN.md` (Phase 4), `docs/COMPONENT-MAP.md`, the `exa_ui` and `exa_game` READMEs
- [x] 5.2 `openspec validate --all --strict`
- [x] 5.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR` (309 passed, 3 ignored)

## Context

How the original draws its HUD. The game interface's HUD routine runs every
frame with the window's size and the GUI scale gs. With a player body, it
draws, if the bars' fade F is above zero: both frames, both drop shadows,
the health bar and the focus bar, at x = W/2 − 50·gs and 52·gs further, y =
H − 5·gs, 48·gs wide; then, if the gauge's fade is above zero, the stamina
gauge 24·gs left of the centre, 9·gs above the bottom, 2·gs tall, with its
frame. Before the bars it draws the heartbeat border while one is running.

- Bars: fill rows y + 2 to y + 4gs − 4. A three-segment bar takes two
  fractions a and b of the width: the first segment from x to trunc(x +
  a·w), a one-pixel line for the pixel where it ends with each colour mixed
  from the first segment's towards the second's by the uncovered share,
  the second segment to x + b·w and the third to the end. Each segment is
  two vertical gradients (top half from its first colour to its second,
  bottom half from its third to its fourth) from a static table. The
  health bar uses yellow, brown, red with a = interpolated yellow / max and
  b = red / max (0 at red ≤ 0.01); the focus bar blue, grey, purple with
  a = focus and b = capacity, its first segment drawn only when at least a
  pixel wide. Fills blend at F².
- Inner shadow: black from round(240·F) at the bar's edge (rect x, y + 1,
  w, 4gs − 4) to 0 at gs pixels in, along the nearest edge.
- Drop shadow: the "Scale" shader outside the rect (x − 2, y) to (x + w +
  2, y + 4gs − 2), 2·gs pixels wide: black, round(220·F)·(1 − r)⁴, r the
  distance from the rect over 2·gs (round at the corners).
- Frame: the rect's border pixels in the HUD skin's colour (129, 134,
  135), the ring one pixel in in (28, 30, 30), both at round(239·F), drawn
  in the skin's texture mode: multiplied by `guitex.rfi` sampled in
  screen space, one texel per pixel, rows from the window's bottom.
- Fade: shown while in combat mode, or in a frame where the drawn yellow
  differs from last frame's or the focus dropped (also while casting, for
  1 s after red health is restored or a quick-bar power, and for 5 s after
  the player interacts with its own body). On the first shown
  frame the rise ends after (1 − F)·500 ms; on the first hidden frame the
  fall ends after F·2000 ms if F < 1, else after 2200 ms with F = 1 until
  the last 2000. Game time. The gauge fades the same way, shown while a
  shield is held in combat mode.
- Heartbeat: on a non-lethal blow that leaves yellow below 0.05, the
  player's heartbeat starts when the simulation clock has passed the next
  allowed time, which becomes now + 60 s; the game modes set that time to
  never for practice and arena career and to 0 for story. The border:
  (80, 0, 0) at round(p²·96), p = 1 − |400 − e mod 800|/400, e ms of real
  time since the start, from the screen edges to round(0.2·H) in on every
  side, gone after 7000 ms; the sound loops at gain 0.3 meanwhile.

The research notes, with addresses, are git-ignored:
`context/re/hud.md`, `combat-look-plan-final.md` (L14), `combat-damage.md`
(the health fields and the block path), `component-map/sections/powers.md`
(focus). The interface page this change extends is
[`docs/UI.md`](../../../docs/UI.md).

## Goals / Non-Goals

**Goals:**
- the bars as the original draws them, from Exarust's health, at the GUI
  scale, with the original's fade;
- the gauge and the heartbeat by the original's rules, within what
  Exarust's game has;
- a close match to the retail screenshot in position, colours and
  proportions.

**Non-Goals:**
- focus, powers, shields, block stamina; other HUD parts; game modes;
  the heartbeat's fade on transitions; second wind.

## Decisions

- **Two crates.** `exa_ui::hud` and `exa_ui::heartbeat` own the painting,
  the fades and the nodes; `exa_game::hud` reads the player into a
  `HudBars` resource, decides the heartbeat and plays its sound. The rules
  (layout, fade, painter, pulse, trigger) are pure functions tested
  without an app.
- **Paint on the CPU, one image.** The original's layers overlap (shadows
  of both bars, the gauge's frame over the bars' shadow) and are blended
  in gamma space; compositing them in order on the CPU reproduces that
  exactly, and the result is one image node in physical pixels. It is
  repainted only when the values, the fades, the window or the scale
  change (every frame only during a fade: 524 × 46 pixels at gs 5).
- **Opacity remap over the floor.** Bevy blends in linear light. Each
  pixel's opacity is fitted (least squares over the channels) so that
  the linear blend gives the original's gamma-space result over the
  arena's dark floor (40 of 255); black (shadows) is then within 3.5
  levels over floors from 10 to 60. The world cursor keeps its own remap.
- **Premultiplied interface output.** The interface camera (order 10,
  no clear) draws into its own texture, cleared to transparent black, and
  Bevy lays a later camera's texture over the earlier ones with straight
  alpha, so a translucent pixel came out multiplied by its opacity twice.
  Measured in game with a test pattern: half-transparent white over a flat
  grey node 152 (predicted 152), over the floor 87 instead of about 150. The
  camera's output now uses premultiplied blending (its colours are already
  multiplied, having been blended over transparent black), and its clear of
  the window stays the default (only the frame's first writer clears).
- **Heartbeat border at an eighth of the size.** It is a linear ramp from
  the edges; an image an eighth of the window's size, linearly filtered,
  keeps it, and is repainted only when the pulse's opacity changes.
- **Focus from the closest existing value.** Exarust has no powers: focus
  and capacity are 1, which is what the original shows for a character
  who never casts (full blue, no grey or purple). The gauge keeps its
  inputs (shield, stamina) in `HudBars` with no shield set.
- **Story mode by level name.** Exarust has no game modes; the heartbeat
  is allowed in every level whose name does not start with "arena"
  (practice and the arena career are the original's modes without it).

## Risks / Trade-offs

- The frame lines' texture offset (the original's texture position comes
  from the last form drawn) is not known for the HUD: Exarust samples the
  stone from the window's bottom-left corner. The lines' mean brightness
  is unaffected; their mottling pattern may differ.
- Which column a vertical line exactly between two pixels lights depends
  on the GPU; Exarust puts the mixed pixel where the first segment ends,
  as the original intends.
- The remap is exact over the floor only; over bright ground the bars'
  shadows come out lighter than the original's.

## Open Questions

- The frame lines' brightness on a lossless retail screenshot (the
  reference is a JPEG: its 1-pixel lines read about 62 and 17 over the
  floor where the rule gives 56 and 14 at the stone's mean): a PNG from the retail game
  in combat mode would settle it (retail request).
- Whether the stone texture's offset matches: same screenshot.

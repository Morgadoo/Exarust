## Context

How the original draws its cursor in the game's view. Its game interface
calls a cursor routine every frame while the game view is up. The routine
reloads the cursor texture when the size changes: the resource pack's
`cursors.rfi` resampled to 2s × 4s pixels with
s = min(round(3 × GUI scale × cursor size) × 2, 128) (cursor size 1 unless
set), a quad half-size of s/2, linear filtering and no mipmaps. It reads
the pointer's position, picks the frame by the player behaviour's cursor
mode, the colour by combat mode, and draws the quad five times: four black
copies one pixel left, up, down and right at alpha (a·(1 − fade))², then the
sprite in colour.

- Frames by mode, as texture offsets: ring v 0.75, one chevron v 0.5, two
  chevrons v 0.25, arrowhead v 0 and the remaining modes, cross u 0.5 v
  0.25, crosshair u 0.5 v 0. Each frame is a quarter tall and half wide.
  The quad's top corners take the frame's higher v: the stored rows run
  from the drawn sprite's bottom to its top, as the bitmap font's glyphs do.
- Turn: for the ring and chevrons a matrix whose Z row is the unit screen
  vector from the pointer to the player's projected point and whose X row
  is up × Z: the sprite's drawn top points away from the player. The
  arrowhead is turned −33° about the up axis and moved by (0.275 s,
  0.4 s), so its tip lies on the pointer.
- Colours in the image: red (255, 76, 76, 180), amber (255, 205, 76, 255);
  a mode other than the ring or arrowhead in combat mode writes 255 into
  the red's alpha byte. Default mode without a player: the arrowhead.
- Option flag (written only by the options screen, 0 by default and in the
  user's game): pins the cursor to the 80 cm plane and warps the system
  pointer. Off: the cursor is simply drawn at the pointer.
- GUI scale: the window-size handler maps the client height to an index
  (0 below 700 px, then +1 at 870, 1051, 1390 and 2080), adds the GUISize
  offset (0) and clamps to 0–5; the scale routine maps the index to 3, 4,
  5, 6, 8 or 12.
- Shapes: the combat branch of the player behaviour tick sets the mode
  only while the condition has no collapse bit: chevrons for a walk
  request (two when the cursor point is over 300 cm away), else the
  arrowhead while the interact flag is set (Ctrl or a pending interaction),
  else the ring; a down character makes no walk request. The exploration
  movement request sets the cross while down, collapsing or dead,
  chevrons for a mouse-driven walk, else the arrowhead when the scene
  point under the pointer is within reach (300 cm across, 300 cm up or
  down, plus a 245 cm term) and the cross otherwise.

The research notes, with addresses, are git-ignored:
`context/re/corpses-and-markers.md` §3 (and its appendix of this change),
`combat-look-plan-final.md` (L4), `player-control.md` §1–§3. The interface
page this change extends is [`docs/UI.md`](../../../docs/UI.md); the
player page is [`docs/PLAYER.md`](../../../docs/PLAYER.md).

## Goals / Non-Goals

**Goals:**
- the original's cursor, sized, coloured, outlined and turned as the
  original draws it, in place of the system pointer;
- its shape from the player's state as the original picks it, within what
  Exarust's player can do;
- the GUI scale as a shared resource for the interface;
- the cursor's point on the 80 cm aim plane, correct under supersampling;
- unattended checks of every shape.

**Non-Goals:**
- the pin-and-warp option, GUI states, interaction targets, the menu
  cursor; combat-mode walking to the cursor; the collapse.

## Decisions

- **Two crates.** `exa_ui::cursor` owns the atlas, the drawing and the
  system pointer; `exa_game::cursor` reads the player and writes a
  `WorldCursor` resource (active, shape, combat, the player's screen
  point) before the draw system's set. The shape rule is a pure function
  of a small state, tested without an app.
- **Bevy UI nodes.** Five absolutely placed image nodes (four outline
  copies under the sprite, global z 1000 and 1001), sized in physical
  pixels (logical ÷ scale factor) as the frame-rate readout is, turned by
  `UiTransform` (clockwise for positive angles, screen y down).
- **Resample on the CPU.** Each frame is cut, turned upright and averaged
  to s pixels by area, then uploaded; the original's resampler is not
  traced, and an area average keeps the thin rims smooth.
- **Gamma-space blending.** The original's interface blends sRGB values;
  Bevy blends in linear light. Measured on the user's retail screenshots:
  the arrowhead's brightest rim pixel 205 against Exarust's 228 linear,
  the ring's rim 160 where linear blending of the same coverage gives
  about 200. So the opacities are remapped: the sprite's to (coverage ×
  alpha)^2.2, exact over black, split between the image and the tint; the
  black outline's to 1 − (1 − coverage × alpha)^2.2, exact over any
  background, baked per outline alpha.
- **The viewport.** A supersampled scene camera renders into an image
  twice the window's size, and `viewport_to_world` takes positions in that
  image. Window positions are scaled by the image's logical size over the
  window's (and back for the player's screen point). This also fixes
  walking to the cursor and combat facing, which aimed at the wrong point.
- **Reach.** The pointer's ray is cast at the level's fixed colliders
  (200 m) and the hit measured from the player's aim point. The 245 cm
  term of the original's test is left out until it is understood.
- **System pointer.** `CursorOptions::visible` is false exactly while the
  world cursor is drawn (play mode with the chase camera, after the load,
  the pointer in the window). The pointer is never moved.

## Open questions

| question | how it will be answered |
|---|---|
| the original's resampler and whether it keeps the ring's rim brighter than an area average | REA on the image resample routine's inner loops |
| what the 245 cm term of the reach test measures | REA on the scene pick routine's output |
| why the dead player's cursor becomes the arrowhead (the death menu?) | REA on the writers of the behaviour's interact flags; Exarust copies the observed result |
| the cursor with Ctrl held in combat mode and out of it, and dying out of combat mode | retail request 4 of the combat-look plan |

## Risks / Trade-offs

- [The sprite's blend over bright ground] → it comes out a little fainter
  than the original's; the floors under the cursor are dark.
- [Rotated rims blur under bilinear sampling] → as in the original, which
  samples linearly too.
- [Other lanes change `player.rs`] → the change only adds the aim plane,
  the viewport helpers and three visibility changes there.

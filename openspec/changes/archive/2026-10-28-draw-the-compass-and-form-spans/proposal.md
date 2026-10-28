# Proposal

## Why

docs/PLAN.md, Phase 4 (combat and items), the game GUI paragraph (the
loot window, power slots and HUD of the gamegui lanes): the HUD's compass
had a place and an opacity rule but was never drawn, and several form and
game-unit rules (the client spans, the set-controls plan, the draw passes,
the counter's segments, the bar and gauge frames) were missing or only
partly reproduced.

## What Changes

- `exa_ui::hud_extra` draws the compass from `compass.rfi`: the frame the
  original builds from the camera's backward axis laid flat (x negated,
  normalised with the approximate reciprocal root) and its cross product
  with up, the origin three pixels against that axis; the picture is
  upright and unmirrored and turns clockwise with the yaw, at
  `round((1 − fade) · 192)`, bilinear-filtered, above the death fade and
  under the bars.
- `exa_game::hud` shows the compass while the player's pack carries a
  usable item of kind 4, sort 11 (the item catalogue now reads each
  record's usable section) and passes the chase camera's back axis.
- `exa_ui::form`: the client spans keep the border at least the frame's two
  stretches and remember it; the set-controls plan (adopt before the
  count's sign, owners for a stored count of 0); the two control draw
  passes and the fade's draw plan; the held-record move and title drag;
  the horizontal scrollbar's step and thumb with the vertical scrollbar's
  inset and a fixed smallest thumb of 9.
- `exa_ui::game_gui`: the counter's backs (`block + 1` apart, each with an
  inner shade at 0.8), fills and outlines (inset by the record's row
  margin); the bar and gauge frames; the game skins, compass install,
  skin shutdown and HUD resets as tested plans; the menu cursor's size
  from the CursorSize setting.
- Review: the compass was mirrored, turned the wrong way and missed the
  3 px shift; a duplicate level-curtain draw (the death fade already draws
  it) was removed; the ease's product order, the spans' border/width mix-up,
  an invented `2·unit + 1` horizontal thumb, the HUD clocks' wrongly
  "32-bit" differences (they are 64-bit signed) and the counter backs'
  one-pixel step (an index register the pseudo-code dropped) were fixed.

## Evidence

The game unit's compass draw (754 bytes) builds a 3×4 frame from the
camera node's third row, crosses it with up, adds −3 times that row to the
origin and emits four vertices with texture coordinates (1,1), (0,1),
(0,0), (1,0) through the engine's point transform; the engine's
normalise uses RSQRTSS. The form's span routine (348 bytes) writes the
border field; the horizontal metric reads a global that is never written
(9). The HUD clocks are loaded with 32-bit moves and subtracted in 64-bit
registers. The counter draw's back step is `lea eax,[esi+r12d]` in the
objdump listing.

## Out of scope

The interface fade value for the compass (0 for now), the status marker's
and skill-ready alert's glow and icon, the power bar on the HUD, frame
dragging of live screens, the HUD's dialogue and message layers, the OS
menu cursor, the live set-controls table copy and set-up pass.

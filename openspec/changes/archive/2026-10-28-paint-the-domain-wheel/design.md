# Design

- The paint is a pure function in `exa_ui` (`wheel_draw`). It takes a
  `WheelDrawView` that the tree builds from its child
  (`TreeScreen::wheel_draw_view`). The view reads the wheel's own
  routines: the eased focus value, the choice flash (which drops a faded
  choice) and the label fade (which drops a spent label). Both clocks are
  the running screen's one clock.
- The quads the original sends without their own texture coordinates
  reuse the default corner set that the backdrop wrote into the same
  vertex slots. The bands and glows therefore carry (0, 0), (1, 0),
  (1, 1), (0, 1).
- The skin inks the paint reads are the line skin's second and third inks
  (`skin::SKIN_INK` as constructed) and the label group's eighth shade.
- The glow program writes no alpha. The canvas takes the alpha as zero, so
  that pass adds (a labelled guess). GLSL's `min` and `max` return the
  operand that is not NaN (a guess; the language leaves it open).
- `Batch` gains a blend word (default `0x203`). Batches only merge when
  their blend words match.
- The resample keeps the shipped picture in `TreeSheets`, so a change of
  GUI unit resamples without reading the pack again.

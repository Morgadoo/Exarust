# Proposal

## Why

Phase 7 (Game modes and content) carries the thaumaturgy tree screen
(`TThaumTree`). A DeepSeek lane (thaumtree5) stalled after two commits
and wrote no report. Its commits added the node and link-turn drags, the
spline's rise terms rounded from double, the GUI level's font for the
tree's text, skipped icon sheets that are not installed, the wheel's
confirm sound and the constructor's defaults. The review read each rule
in the instruction listing. It found these faults: the node drag's tenth
was taken in single precision, and the link-turn drag read a new field
that nothing set (the screen's focus id already holds the link). The draw
cleared the refusal flag after one frame instead of when the flash ends.
The label box was centred by the text's width, not by the box's. The
focused prompt caption used an invented outline colour, although the
skin's glowing text was already in Exarust. The spline count's 64-bit
conversion was saturated, and the wheel texture's sizes were hard-coded
for one GUI unit and documented wrongly.

## What Changes

- The node drag places the node at the pointer less the grab offset. It
  works the base place out as `(placed − pan)/zoom` in single precision
  times a tenth in double.
- The link-turn drag takes the link from the focus id (bits 16 to 19, bit
  21 incoming) on the pressed node. The outgoing turn is measured about
  the owner; the incoming turn is measured about the node the link ends on.
- The draw ends a flash whose time is up and clears the refusal with it.
- The spline's sample count is `round(...)` into 64 bits, plus one, with
  the low 32 bits held to 2..30.
- The label box is centred by its own width again.
- A focused prompt caption is `skin::glow_text` with the line skin's inks.
- The wheel's confirm plays `ThaumShow02` at 0.25 and pitch one (verified).
- The wheel texture is 32 units a side and its size word is 8 units.

## Evidence

The tree's mouse-move handler, the focus hit test, the link spline
routine, the dragged-link routine, the label and prompt passes of the
screen draw, the draw head's flash fades and the constructor, all read in
the instruction listing. Also read: the domain wheel's confirm, the skin's
glowing-text routine and the wheel texture load in the executable's
listing. The listing drops the `r12d` index of the prompt's focused y
(`lea edx,[eax+r12d]`); the executable's listing has it.

## Out of scope

The running screen does not resample the wheel sheet to 32 units, and
nothing enters the node and link-turn drag states yet (no press in the
pack sets them). Icon sheets are taken as square. The non-focused prompt
caption's skin colour and GPU submission of the tree's draw are also out
of scope.

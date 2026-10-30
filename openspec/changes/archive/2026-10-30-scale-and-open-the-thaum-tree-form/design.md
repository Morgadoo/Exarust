# Design

## Wheel show value versus bar fade

The draw head computes the opening ease `1 − r²` and writes it, times the
window when the closing fade runs, into the domain wheel child's shown
value. The bar is drawn by a separate callback that takes only the form's
width and reads its own animation flags and stamps, so its slot alphas are
the opening and closing curves alone. Exarust renames the field
`wheel_shown` and removes the multiply from `bar_pose`.

## Interface unit in the hit tests

The slot test and the prompt test read the interface unit global. The
screen already draws at `GuiScale::scale`; the pointer system passes the
same value (four when the resource is absent) so drawing and hit testing
agree. The arithmetic stays integral as in the original.

## Opening words

The opening routine is called with no form by the creator (mode 3, task
word `0x10`, hint byte `0xff`) and with the game's form otherwise (task
word `0x1010`, the form's child kept). These are fields on `TreeScreen`;
`overlay_panel_alpha` returns the child's paint alpha
`shown − 2·window`, unclamped, or none for the creation screen.

# Design

## Behaviour

Records carry a class and id, an x that is pixels or an anchor, a y in 4-px grid units, w/h with skin defaults, text, style and state. The resolver keeps the previous row: a y anchor adds its delta to the previous y (negated with style bit 0x80000); an x anchor reuses or offsets the previous row's right edge. Width and height fall back to the class's skin size and round to a 4-px grid. The form scales the unit-4 design by `unit/4`.

The tab array has SETTINGS/CONTROLS, Keyboard/Controller sub-tabs, two separators and Defaults/Apply/Cancel. The settings array has the two info rows, the audio slider with Music, seven choice rows, four checkboxes and Early Checkpoints. The bindings array has 22 label rows and 21 key cells; the list content is 676 px inside a 416-px view.

## Model

`exa_ui::settings` exposes the record types, the resolver and the complete `Layout`.

## Open questions

The scroll-wheel step, the tab page bodies and the settings apply/reset logic.

## Docs

`TGUIForm` is **15%**. The headline rises slightly.

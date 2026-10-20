# Design

## Behaviour

The page switch returns the visible child and its rectangle. Page 0 shows Settings at (0,64,426,408). Page 1 shows Controls at y 56 on a widescreen layout (height 416) or y 96 otherwise (height 376). The Controls scrollbar uses the 412-px client of the 416-px child, 676-px content, a 15-px rail and a 9-px minimum thumb; the thumb is 232 px and its y is `17 + 150·f`. The unit-4 values scale by `unit/4`.

## Model

`exa_ui::settings` adds `PageSwitch`, `switch_page` and `controls_scroll`.

## Open questions

The wheel handler, sub-tab bodies and apply/reset logic.

## Docs

`TGUIForm` is **16%**. The headline rises slightly.

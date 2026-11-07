## ADDED Requirements

### Requirement: Combo box arrow
A combo box SHALL draw, unless its style has `0x8000000`, the small atlas's down chevron at `(4·unit|1) − unit/2 + 1` in from its right edge and `(unit + 1) − (unit & 1)` below its top: plain in the label group's `0xd4` slot when its state has bit 0x10, else outlined at full strength in the edit state (`state & 0xf0000 == 0x10000`), else plain in the `0xe0` slot.

#### Scenario: A disabled box in the edit state
- **WHEN** a combo box's state is `0x10010`
- **THEN** its arrow is the plain chevron in the `0xd4` slot

### Requirement: Combo box text
A combo box's text SHALL be the run its creation fits into the span `w − (4·unit|1) − 2·pad − unit` from the start, placed by the label alignment over that span with the run's width and the main font's ascent, and drawn in the `0xe4` slot.

#### Scenario: The loadout form's fighter list
- **WHEN** a 220-pixel combo box with pad 1 and style `0x8403320` at unit 4 shows a 30-pixel name at 100, 50, 24 high, with a font of ascent 14
- **THEN** it draws no arrow and its name's pen is at 188, 70

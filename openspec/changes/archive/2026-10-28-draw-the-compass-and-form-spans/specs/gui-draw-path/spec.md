## ADDED Requirements

### Requirement: HUD compass
The HUD SHALL draw the compass while the player carries a usable item of kind 4, sort 11, as a square of half extent `12·unit` centred `14·unit` from the view's left and bottom, at an opacity of `round((1 − fade) · 192)` of 255, turned by the frame of the compass frame requirement.

#### Scenario: Shown with the item
- **WHEN** the player's pack holds an item whose usable section is kind 4, sort 11 and the HUD is active
- **THEN** the compass is drawn, and at fade 0.5 its opacity is 96 of 255

### Requirement: HUD compass frame
The compass frame's axis `d` SHALL be the camera's backward axis laid flat with x negated and normalised, its first axis `d × up`, and its origin the centre plus `−3·d`; the local corners `(−e,−e)`, `(e,−e)`, `(e,e)`, `(−e,e)` SHALL take texture coordinates `(1,1)`, `(0,1)`, `(0,0)`, `(1,0)`.

#### Scenario: Camera looking down −z
- **WHEN** the GUI unit is 4, the view is 900 pixels tall and the camera's backward axis is (0, 0, 1)
- **THEN** the corner with texture (1,1) is at about (104, 793) and the picture's top-left texel is painted near (9, 794)

#### Scenario: A quarter turn
- **WHEN** the backward axis is (1, 0, 0)
- **THEN** the corners lie about (107, 892), (107, 796), (11, 796), (11, 892) and the picture's top-left texel is painted at the square's top right

### Requirement: Form client spans
Setting a form's controls SHALL raise its border to the sum of the frame's stretches (style bits 0x30 and 0xc0) when that sum is larger, otherwise remember the first border and restore it on later calls; the client's right and bottom edges SHALL be the width and height less the new border and the skin's scroll inset for each scrollbar attribute, and the title SHALL be the skin's height plus 2 (attribute bits 0x20 without 0x800000) or the top stretch, or the new border without a title.

#### Scenario: Stretches above the border
- **WHEN** a 302 × 402 form with border 2, style 0xc0, both scrollbars and a title sets its controls with stretches 8 and 9, title 20 and inset 7
- **THEN** the border is 9, the right edge 286, the bottom 386 and the title 29

### Requirement: Counter segments
The counter control SHALL draw `segments + 1` backs `block − 1` wide and `block + 1` apart from one pixel in, where `block = round(width / (segments + 1) − 1)`, each with a shade two pixels in at 0.8, and fills from two pixels in that end at the first share at or below zero.

#### Scenario: Four segments at half
- **WHEN** a counter of 4 segments at 0.5 is drawn at (10, 20), 100 × 16
- **THEN** the backs start at 11, 36, 61 and 86, 23 wide, and the fills are (12, 21, 21, 14) and (37, 21, 21, 14)

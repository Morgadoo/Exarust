# Spec Delta

## ADDED Requirements

### Requirement: Basic control class draws
Each basic control class SHALL make the skin calls its dispatch makes: a button its shell, a check box the skin's panel face, an image button its frames then the box, a trackbar its frames then the slider, a label its panel then its frames, and an icon button and a picture their frames; the image button's front SHALL go through the skin's image draw and an icon button's own-atlas icon SHALL draw without an ink while pressed.

#### Scenario: Check box back
- **WHEN** a check box's back is drawn
- **THEN** the call is the panel face, not the box

#### Scenario: Pressed image button
- **WHEN** an image button of 28 pixels with the shared 22-pixel atlas is pressed and lit
- **THEN** its image draws as it is at scale 1, at (3, 4)

### Requirement: Held label fade
A held label SHALL fade its sixth text shade's alpha by the elapsed clock over 300 in double precision for a third of a second, take the fourth shade whole while focused, and past the fade clear its held bit and draw plainly; an unordered fade SHALL keep fading.

#### Scenario: Half way
- **WHEN** a held label with a sixth shade of alpha 70 is drawn 150 ms after its first draw
- **THEN** it draws in that shade at alpha 35 and asks for another frame

#### Scenario: Past the fade
- **WHEN** it is drawn 400 ms after its first draw
- **THEN** its held bit is clear and it draws through the plain slot matrix

### Requirement: Table choice list placement
The table's choice list SHALL ask the general form placement for its own x, or the x plus the table's width, and for a top the row height less one below its corner, or a bottom one pixel below the corner; with the skin's first flag bit set the corner SHALL move up a unit less one first.

#### Scenario: Under the header row
- **WHEN** a list at (105, 5) opens from a table 300 wide with a 30-pixel row
- **THEN** the request is x 105 or 405 and y 34 or a bottom at 6

### Requirement: Colour picker callbacks
The colour picker's control callbacks SHALL write the RGB or HSV colour (a slider over 255 or raw) and refresh from it, take the field's two values as hue and saturation, take a colour control's bytes, finish with the word 0x1000, and revert to the colour the swatch had when the picker opened.

#### Scenario: Field point
- **WHEN** the field reports 0.25 and 0.75
- **THEN** the picker's hue and saturation are 0.25 and 0.75

### Requirement: Drop-down selection band
The drop-down list's selected row SHALL draw a double frame in the hot form group's seventh and first shades, a half-strength gradient from the form group's fifth shade to its first, and the skin's unselected selection band over the inner rectangle.

#### Scenario: Second row
- **WHEN** the second row of a unit-4 list is selected
- **THEN** the band's outline starts at (2, 26)

### Requirement: Inline picture corners
An inline text picture SHALL emit its corners bottom-left (u 0, v 0), top-left (0, 1), top-right (1, 1) and bottom-right (1, 0), under the premultiplied blend.

#### Scenario: A 50-pixel picture
- **WHEN** a picture spans x 25 to 75 and y 0 to 50
- **THEN** its first corner is (25, 50, 0, 0) and its third (75, 0, 1, 1)

### Requirement: Physique cross unordered point
The physique cross's weight split SHALL take the right half for a first coordinate above 0.5 or unordered.

#### Scenario: Not a number
- **WHEN** the first coordinate is not a number
- **THEN** the right half's weights are taken

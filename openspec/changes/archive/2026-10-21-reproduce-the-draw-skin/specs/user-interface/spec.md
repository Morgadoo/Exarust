## ADDED Requirements

### Requirement: Skin scheme shades
A scheme colour SHALL expand into twelve shades: the base at 4; the base times the scale, its square and its cube at 5 to 7; at 8 to 11 the base toned by 1.75, 2, 2.25 and 2.5 times the tone (at most 1) and scaled by the fourth to seventh powers; at 3 down to 0 the base over the scale, divided by the scale again, then twice more with a 10/11 step each; and last entries 4 to 7 toned by 0.5, 1, 1.25 and 1.5 times the tone.

#### Scenario: A gentle scale
- **WHEN** 60, 90, 120 expands with scale 1.25 and tone 0.4
- **THEN** shades 0 to 3 are 20, 30, 41; 28, 42, 56; 38, 58, 77 and 48, 72, 96
- **AND** shades 4 to 8 are 65, 89, 113; 87, 109, 132; 113, 137, 160; 146, 169, 193 and 188, 210, 232

### Requirement: Skin tone and scale rounding
Toning SHALL pull each channel towards the luminance (green × 0.587 plus red × 0.299, then blue × 0.114) and keep the low byte of the rounded result; scaling SHALL round to nearest even and saturate; scaled shades SHALL keep the destination's alpha.

#### Scenario: A tone past one wraps
- **WHEN** 200, 0, 100 is toned by 2
- **THEN** it becomes 198, 142, 42

### Requirement: Skin accent shades
The scheme's accent SHALL give a dim shade (half) and a bright one (1.9, then toned by 0.8), both with the second alpha byte times the scheme's float as their alpha, and four mixes at 0.4, 0.6, 0.5 and 0.9 towards the bright one; the last two SHALL be blended from the form group's fourth shade by that product over 255.

#### Scenario: The accent alpha wraps
- **WHEN** the accent is 200, 100, 50 with scale 1.5 and second alpha 200
- **THEN** the dim shade is 100, 50, 25, 44 and the bright one 210, 197, 178, 44
- **AND** the last two mixes have alpha 38

### Requirement: Skin alpha ramps
The first group SHALL be copied to the hot group. The alpha ramps SHALL give the form group and the last two mixes the first byte times its scale, and all forty-eight shades of the control, hot, label and highlight groups the second byte times its scale.

#### Scenario: The last highlight shade ramps too
- **WHEN** the second alpha 160 is scaled by 0.25
- **THEN** every highlight shade, the twelfth included, has alpha 40

### Requirement: Skin batch
Selecting the mode already selected SHALL bind nothing; mode 0x10 SHALL ask the subclass and keep its answer as the selected mode; an unnamed mode SHALL be stored and bind nothing. A run's end SHALL submit its vertices as quads, lines or triangles with the state then current, into one vertex list that a cleared batch reuses.

#### Scenario: Selecting twice
- **WHEN** mode 2 is selected with the tint and again without it
- **THEN** the texture tint stays 1

#### Scenario: Reuse
- **WHEN** a batch is cleared and refilled
- **THEN** its vertex list keeps its capacity

### Requirement: Skin selection band
A selection band SHALL draw an outline whose middle is half the width less one in, then glow fans reaching into the row from its top and bottom edges, wider when selected.

#### Scenario: The lower glow
- **WHEN** a band spans 10, 20 to 110, 100 at unit 4, unselected
- **THEN** the outline's middle is at x 59.5
- **AND** the lower fan reaches from y 99 up to 94 at the middle and 95 at the sides

### Requirement: Skin line boxes
Double frames SHALL start each top edge a pixel short of the right; bevel and gradient line boxes SHALL meet their left mixes half a pixel below the right ones, the bevel's right side ending on its top edge; two-ink bands SHALL be two lines; four-ink frames SHALL fade their sides from the edge inks.

#### Scenario: A bevel box
- **WHEN** a bevel box spans 10, 20 to 30, 40
- **THEN** the left mixes meet at 30.5, the right ones at 30, and the right side ends at 29.5, 20.5

### Requirement: Skin gradients and shadows
A clamped gradient frame SHALL clamp its bands on the rectangle before its lines move in; alpha-scaled gradients SHALL keep the low byte of the scaled alphas; a shadow SHALL halve its height as an unsigned word.

#### Scenario: A gradient frame
- **WHEN** a gradient frame spans 10, 20 to 110, 100 at unit 4
- **THEN** its left bands meet at 10.5, 28 and 10.5, 92

### Requirement: Skin scroll arrows
A scroll arrow SHALL draw six line pairs in a look's inks: look −1 the label group's second shade, look 0 its fifth, look 1 the highlight group's ninth under the additive blend; the copies after the fourth vertex SHALL take the shade before at two thirds of its alpha. Other looks SHALL draw nothing.

#### Scenario: A horizontal arrow
- **WHEN** an arrow of width 9 is drawn at 10, 20 with look 0
- **THEN** its first four vertices are 10, 25; 14, 21; 19, 25 and 14.5, 20
- **AND** a named shade of alpha 200 gives the copies alpha 132

### Requirement: Skin control faces
A face SHALL draw its two-ink rectangle, bordered box and outline in the pressed, hot or idle shades; the gradient face's hot bevel SHALL run from the hot group's fifth to its sixth shade; the pressed panel's border SHALL use control shades 5 and 8.

#### Scenario: An idle face
- **WHEN** an idle face is drawn
- **THEN** its rectangle runs from control shade 5 above to 3 below and its border from 8 outside to 6 inside

### Requirement: Skin sliders and scroll bars
A slider's track SHALL fade the hot group's fifth and first shades at 0.6 alpha and its filled bar SHALL be one unit wide; a scroll bar SHALL be drawn a pixel lower and two shorter, its striped arm at style 0x1000_0000.

#### Scenario: A scroll bar
- **WHEN** a scroll bar spans 0, 0 to 100, 24 at unit 4
- **THEN** its trough is 1, 2 to 99, 22 and its thumb 83, 3 to 98, 21

#### Scenario: A filled slider
- **WHEN** a filled slider of width 100, inset 4 and unit 6 is at one half
- **THEN** the bar's shift is 43

### Requirement: Form marks
A form mark pressed under the held button SHALL draw a bevel just outside its rectangle, the outline and a face a pixel in fading from the form group's fourth shade up to its third; the mark under the mouse SHALL draw that bevel, a second one a pixel inside and the outline.

#### Scenario: A hovered mark
- **WHEN** a mark is under the mouse and not pressed
- **THEN** it draws two bevels and an outline and no face

### Requirement: Form mark icons
A mark's icon SHALL sit half a unit in from the left and half a unit and one below the top, at scale 1 when the mark is under the mouse or pressed with the button up, otherwise in the label group's seventh shade.

#### Scenario: A close mark
- **WHEN** a 200-wide titled form with a close mark is drawn at unit 4
- **THEN** the mark spans 179, 3 to 196, 23 and its icon is at 181, 6

### Requirement: Form chrome
The chrome SHALL draw the gutter strips (none above a titled form), the client fill when the form keeps no lines, the first style field's double frame or (style bit 0x40) an outline a pixel in, and the second field's double frame two pixels outside the client; a flat form SHALL draw only its frame.

#### Scenario: The client frame
- **WHEN** the client spans 20, 30 to 180, 80 and the second field is set
- **THEN** a double frame runs from 18, 28 to 182, 82

### Requirement: Form background scrollbars
The default background's vertical scrollbar SHALL draw its frame, shadows, arrows −9 and −10 as frame slots pressed when held and one gradient bar for the thumb with the texture shifted by its place; the horizontal one SHALL draw arrows −13 and −14 and a frame-slot thumb with the texture shifted.

#### Scenario: The vertical scrollbar
- **WHEN** the thumb is at 30 with length 20 on a 100-high form with client top 24
- **THEN** the arrow frames are 180, 24 to 195, 39 and 180, 80 to 195, 95 and the thumb 180, 30 to 195, 50
- **AND** one shadow is 180, 50 of size 15 by 11

### Requirement: Form background title
The default background SHALL draw the rounded shell, the title gradient, the optional two-line band and the marks before the scrollbars, then the arrow icons, the mark icons and the title's two blocks at y `(margin + line height + 25) / 2`.

#### Scenario: The title
- **WHEN** a 200-wide form with margin 4, inset 5, a close mark and line height 13 is drawn at unit 4
- **THEN** the title blocks are 165 wide at 8, 21 in the form's second shade and 9, 22 in the label's seventh

### Requirement: Form page order
A page SHALL draw its background, stop when its state hides it, clip to its client, translate its contents to the inset and top less the scroll (zero when locked), then draw the line rectangles, the controls, the visible children and last the drop shadow, which the lock does not remove.

#### Scenario: The origin
- **WHEN** a page with inset 5, top 24, right 200 and bottom 130 has no stored scroll
- **THEN** the scroll is −195, −106 and the origin 200, 130
- **AND** locked, the origin is 5, 24

### Requirement: Skin label inks
A label SHALL draw in the label group's third shade when focused and its seventh otherwise, the slot's idle label in its ninth; the slot's pressed label SHALL draw control shade 2 one pixel left and label shade 7 one below; the block form SHALL keep the bytes that fit, add the offset and right-align for style 0x4000.

#### Scenario: A block label
- **WHEN** a 100-wide block label with inset 2 and step 7 is drawn
- **THEN** it keeps 13 bytes

### Requirement: Skin glowing text
A coloured label SHALL draw its text in the label group's eighth shade at the measured place, then four copies a pixel left, right, below and above in the highlight group's first shade under the additive blend.

#### Scenario: The glow
- **WHEN** coloured text is drawn at 10, 20
- **THEN** the copies are at 9, 20; 11, 20; 10, 21 and 10, 19

### Requirement: Skin label fade
A fade SHALL take a zero start as now and scale the label group's seventh shade's alpha by the elapsed third of a second, keeping the low byte, and clear the fade bit once it passes 1.

#### Scenario: A fade
- **WHEN** a label started at 1000 is drawn at 1150
- **THEN** its alpha is half of the shade's
- **AND** at 1400 the fade bit is cleared

## MODIFIED Requirements

### Requirement: Skin frames
At place 10, 20 of size 100 by 80, style bits 0x30 SHALL select one double frame: 0x10 from 10, 20 to 110, 100 in the hot group's first then eighth shade, and 0x20 the same rectangle in its seventh then first. Style bits 0xc0 SHALL independently select a second double frame inset by one on every side, with 0x40 using the first then eighth shade and 0x80 the seventh then first. Any other value in a group SHALL select nothing.

#### Scenario: Each group selects its frame
- **WHEN** the style is 0x10
- **THEN** one frame is 10, 20 to 110, 100 in the hot group's first and eighth shades
- **WHEN** the style is 0x20
- **THEN** one frame is the same rectangle in the seventh and first shades
- **WHEN** the style is 0x40
- **THEN** one frame is 11, 21 to 109, 99 in the first and eighth shades

#### Scenario: Both groups run
- **WHEN** the style is 0x50
- **THEN** the first frame is 10, 20 to 110, 100 and the second is 11, 21 to 109, 99
- **WHEN** the style is 0, 0x30 or 0xc0
- **THEN** no frame is drawn

### Requirement: Skin shader programs
With the guard clear the method SHALL return three programs in order: kind 5, the flat program, with `MPrj` set to 2; kind 0x15, the rounded program, with `MPrj` set to 2 and a `Scale` lookup; and kind 0x15, the textured program, with `MPrj` set to 2, `Tex` set to 0 and a `TexCol` lookup. It SHALL count six uniform operations. With the guard set it SHALL return none.

#### Scenario: The three programs
- **WHEN** the guard is clear
- **THEN** three programs are returned with kinds 5, 0x15 and 0x15
- **AND** the first is the flat program and sets `MPrj` to 2 and nothing else
- **AND** the second is the rounded program, sets `MPrj` to 2 and looks up `Scale`
- **AND** the third is the textured program, sets `MPrj` to 2, sets `Tex` to 0 and looks up `TexCol`
- **AND** six uniform operations are counted

#### Scenario: The guard
- **WHEN** the guard is set
- **THEN** no program is returned and no uniform operation is counted

# gui-draw-path Specification

## Purpose
How the interface's skin batches reach the screen: the blend word, the skin programs, the pixel rules of the CPU rasteriser, the texture rows and the font fallback, and which screens are drawn through it.

## Requirements

### Requirement: Blend word
The draw path SHALL read a blend word's high-byte low nibble as the source factor and its low nibble as the destination factor from the original's ten-entry table, SHALL draw with blending off when the word under the mask 0x3f0f is 0x100, and SHALL draw a word of 0 as 0x203.

#### Scenario: Steady state
- **WHEN** a primitive is drawn with word 0x203
- **THEN** its source factor is source alpha and its destination factor is one minus source alpha

#### Scenario: Blending off
- **WHEN** a primitive is drawn with word 0x100
- **THEN** it replaces the pixels under it

### Requirement: Skin programs
Solid primitives SHALL draw the vertex ink; textured primitives SHALL mix the sampled texel by `TexCol` and multiply by the ink; rounded primitives SHALL cut the alpha with the `Scale` uniform; mode 0x10 SHALL draw what the skin's subclass answers, the flat program for the line skin.

#### Scenario: Line skin subclass
- **WHEN** a mode 0x10 primitive is drawn by the line skin
- **THEN** it draws its vertex ink and samples no texture

### Requirement: Shared quad edge
A pixel on the diagonal two triangles of one quad share SHALL be drawn once.

#### Scenario: Half-transparent quad
- **WHEN** a quad of alpha 128 is drawn
- **THEN** every covered pixel is blended exactly once

### Requirement: Screens from the constructors' words
The inventory and journal SHALL paint with the attribute and frame-style words their constructors write and SHALL draw no title bar, since the title bit is set by no shipped form. The pack panel SHALL use attributes 0x130032 and style 0x80. Control records SHALL be laid out by the two-pass layout inside the client.

#### Scenario: Inventory records
- **WHEN** the inventory at unit 4 is laid out
- **THEN** the Encumbrance label is at client (4, 386) and Primary is 108 by 24 at (4, 416)

#### Scenario: Journal records
- **WHEN** the journal is laid out
- **THEN** New is 104 by 24 at client (4, 20) and Erase is 104 by 24 at (120, 20)

#### Scenario: Title bit
- **WHEN** a frame is painted without the title bit
- **THEN** no title text is drawn

### Requirement: Texture rows and font fallback
A texture coordinate SHALL run up the image, so a texture stored top-down SHALL be sampled from its bottom row, and a text quad's top edge SHALL name the cell's far row. When a level's font texture is not installed, the text SHALL keep the level font's glyph quad, advances and line height and SHALL sample the base font's atlas cells.

#### Scenario: Striped texture
- **WHEN** a quad's top edge is at v 0.75 and its bottom edge at v 0 over a texture of four rows
- **THEN** its top pixel shows the third row from the bottom and its bottom pixel shows the bottom row

#### Scenario: Missing level font
- **WHEN** the level font texture is missing and the base font texture is installed
- **THEN** the screen's glyph quads have the level font's size and sample steps of one base glyph

### Requirement: Item panel icons
The inventory screen SHALL draw each visible pack entry whose icon is in the panel's atlas through the `HltCol` program at its cell inside the pack panel's client area, clipped to that area, and SHALL draw no icon layer for an empty pack.

#### Scenario: One entry
- **WHEN** the pack holds one entry with a cached icon at cell (10, 20) and the inventory opens
- **THEN** the frame holds one `HltCol` layer clipped to the pack's client area with its quad 10 pixels in from the clip's left

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

### Requirement: HUD power marker and skill-ready alert
The HUD SHALL draw the power marker at `(W/2 + 53·unit, H − 8·unit)` and the skill-ready alert at `(W/2 − 60·unit, H − 8·unit)` as the skill-icon atlas's first cell, 8 units square, each over a rounded black halo filling the square from 3 to 5 units into the icon grown by 8 units at alpha `round(0.6 · pulse · 255)`.

#### Scenario: Marker at half pulse
- **WHEN** the GUI unit is 4, the marker sits at (100, 200) and the pulse is 0.5
- **THEN** the halo's quad spans x 80 to 152 in black at alpha 76 and the icon is one 32-pixel quad from (100, 200) tinted (140, 160, 255, 128)

### Requirement: HUD status icon inks
The marker's icon SHALL be tinted (140, 160, 255) at alpha `CVTSS2SI(pulse · 255)`, and the alert's icon SHALL be drawn outlined at the pulse, its face in the skin's seventh label ink and its four one-pixel copies in the third highlight ink.

#### Scenario: Alert at full pulse
- **WHEN** the alert is drawn at (40, 60) with the pulse 1
- **THEN** the face quad starts at x 40 in the seventh label ink and the first outline copy at x 39 in the third highlight ink

### Requirement: HUD status pulse clock
The marker and the alert SHALL share one pulse clock: the alert's first frame stamps it with the clock, the marker reads it as it is (zero before any stamp), a frame without a player draws neither and keeps the stamp, and only the HUD switched off clears it.

#### Scenario: The alert stamps the clock
- **WHEN** the marker draws alone at 1500 ms and the alert first shows at 3000 ms
- **THEN** the marker's pulse is 0.5 at 1500 ms and 0 at 3000 ms, the alert's pulse at 3000 ms is 1, and at 4000 ms it is 0

### Requirement: Compass interface fade
The compass's fade SHALL be the open power tree's window fade, and 0 while the tree is shut.

#### Scenario: Tree open at a quarter
- **WHEN** the power tree is open with a window fade of 0.25
- **THEN** the compass's fade is 0.25, an opacity of `round(0.75 · 192)` = 144

### Requirement: Form close cascade
Closing a form SHALL restore the controller's action set for style bit 0x20 even when already closing; a form not yet closing SHALL mark itself when it closes once; the task's active child SHALL run its callback, clear the task's active child and close the task; any other form SHALL clear the task's remembered child when it is this form, close the task's children it owns that are not closing, then its own children not closing, then hide, run its callback and redraw.

#### Scenario: Owned popup and child
- **WHEN** a window owning one popup in its task and having one child not closing is closed
- **THEN** the popup is closed and hidden first, then the child, then the window hides, runs its callback and redraws

### Requirement: Form maximise and restore steps
A maximise SHALL show the form, save its record, tell the window-state hook word 2 before the form changes, write the place, set the state's low bits to 2, resize, run the layout-changed virtual and redraw; a restore SHALL show the form and, with a record, write it back, resize, free it and refresh the control spans, then tell the hook word 0 with or without a record, clear the state's low bits, run the layout-changed virtual and redraw.

#### Scenario: Restore without a record
- **WHEN** a form without a maximise record is restored with a hook installed
- **THEN** it is shown, the hook is told 0, the state's low bits clear and the form redraws without moving

# Spec Delta

## ADDED Requirements

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

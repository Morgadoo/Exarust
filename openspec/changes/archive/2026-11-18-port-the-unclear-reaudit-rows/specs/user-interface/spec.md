## ADDED Requirements

### Requirement: Parchment block sample
The parchment's hit test SHALL read the compressed 4x4 block holding the click's bottom-up storage row (the height less the row) and column, and answer the block's texel at column 1, row 1; a texel past the image is transparent.

#### Scenario: A click inside a block
- **WHEN** a click lands anywhere in a block whose (1, 1) texel is opaque
- **THEN** the parchment takes the click

### Requirement: Skill screen armour refresh
Closing the skill screen SHALL write the technique word and then run the armour pass over the player's worn outfit.

#### Scenario: Closing the skill screen
- **WHEN** the skill screen closes
- **THEN** the player's body is marked for the armour refresh

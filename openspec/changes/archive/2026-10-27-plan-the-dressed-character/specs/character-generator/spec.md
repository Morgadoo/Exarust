## MODIFIED Requirements

### Requirement: Merge order and poses
The system SHALL visit a garment's chosen nodes for the skin weights along the slot tree, append the dressed mesh's geometry in slot order 1 to 18, and restore a saved bone tree by matching each record to its parent bone's nearest child within 20 cm, skipping a record's subtree when none matches.

#### Scenario: Merge order
- **WHEN** every slot has a chosen node
- **THEN** the weight pass visits them in the order 1 to 13, 17, 14 to 16, 18

#### Scenario: Append order
- **WHEN** a garment has chosen nodes in slots 13, 14, 17 and 18 and every slot is visible
- **THEN** they are appended in the order 13, 14, 17, 18

## ADDED Requirements

### Requirement: Dressed mesh list
The system SHALL list the dressed mesh as each garment's chosen nodes, in layer-level order, gated by the slot's select mask against the garment's visible mask and carrying the slot's cull bits (`0x400000` when the second cull mask misses the visible mask, else `0x200000` when the first does), then the body's slots whose select mask meets the body's visible mask without cull bits, then the overlays; a race whose look word lacks bit `0x4` appends every body slot ungated.

#### Scenario: Body only
- **WHEN** the body-only list is built for an empty slot table and one overlay node
- **THEN** it holds the overlay alone

### Requirement: Impact material
The system SHALL give each body slot the material byte of the garment covering it whose uncovered word is the largest, compared as an unsigned 64-bit value with a strict less-than so the first garment in layer order keeps a tie.

#### Scenario: Outer garment
- **WHEN** an inner garment on chest layer 0 and an outer one on layer 1 are worn
- **THEN** the chest takes the outer garment's material

### Requirement: Feral underwear
The system SHALL paint the underwear (item 8) with damage and dirt `(2 − f)·f·255` converted to the nearest even integer and cut to a byte, keeping its quality, and seed it with the record's seed; any other first apparel keeps its appearance and seed.

#### Scenario: Half feral
- **WHEN** the feral level is 0.5
- **THEN** damage and dirt are 191

### Requirement: Attach pass
The system SHALL hang the main hand's weapon and then one off-hand item: a shield from either hand first, else hand 1's torch, else hand 1's weapon; only a weapon reads its hand's lit flag, and the last weapon's flipped flag is the character's grip.

#### Scenario: Shield in the main hand
- **WHEN** hand 0 holds a shield and hand 1 a sword
- **THEN** only the shield is hung, on the off arm

### Requirement: Rebuild apply step
The system SHALL clear the texture mark of the first direct mesh child carrying a skin job, register a newly built tree, and on a hair rebuild remove the head's hair-owned direct children and attach the new roots.

#### Scenario: Hair elsewhere
- **WHEN** a hair-owned node hangs under the root rather than the head
- **THEN** the apply step leaves it in place

# Spec Delta

## ADDED Requirements

### Requirement: One painted stem per garment
A garment model SHALL be painted from a single stem: the first run-time `&` material met walking depth first (a node's parts in order, then its children in order, from each root in turn), and every `&` part of the model SHALL take that stem's painted material.

#### Scenario: Two stems
- **WHEN** a model's first node carries `&doublet_ba1` and a later node `&otheraaaa1`
- **THEN** both parts are painted from stem `doublet_ba`

#### Scenario: Child before the next root
- **WHEN** root 0 has no `&` part, its child has `&childstem1` and root 1 has `&secondroo1`
- **THEN** the stem is `childstem1`

### Requirement: Character skin composite
The skin composite SHALL multiply the colour texel by the tone, pack the surface as (1, min(0.8·gloss, 1), 0), add each bound normal layer's (an − bn)·amount to the base, bind at most five layers stopping at the first amount not above zero, and draw the beard overlays only over the map's first quarter-by-quarter corner with the whole beard map stretched over it.

#### Scenario: One texel
- **WHEN** colour (204, 166, 146), tone (0.46, 0.47, 0.47) and gloss 128 are composed
- **THEN** the colour bytes are (94, 78, 69) and the surface bytes (255, 102, 0)

#### Scenario: Beard corner
- **WHEN** a half-alpha beard is composed over a black skin with gloss 0 on an 8×8 map
- **THEN** texel 0's surface is (133, 6, 6) and texel 2 keeps the skin's (255, 0, 0)

### Requirement: Merged skin mesh runs
The merged skin mesh SHALL group vertices by material in first-seen order, each run's vertices in appended order, each run starting right after the previous run's last vertex.

#### Scenario: Two materials
- **WHEN** a three-vertex body part and a three-vertex garment of another material are merged
- **THEN** the mesh has six vertices, the second run starts at vertex 3 and the indices are 0…5

### Requirement: Apparel matching with parent points
A garment seam group SHALL start at its first joint's distance, parent group points SHALL only lower that bar, and a later joint SHALL be taken only when strictly nearer than the bar; a parent point SHALL never be taken.

#### Scenario: Shadowed joint
- **WHEN** joint 0 is 50 cm away, joint 1 10 cm and a parent point 1 cm
- **THEN** the group keeps joint 0

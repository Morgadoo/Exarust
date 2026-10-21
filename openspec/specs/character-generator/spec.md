# character-generator Specification

## Purpose
The original's character generator, which builds a body from its record
and race: the body slots and apparel layering, the skin job's look, hair,
the body tree's passes, the rebuild, the equip pass and the spawns.

## Requirements

### Requirement: Slot table
The system SHALL model the generator's eighteen body slots as the executable's slot records give them: each slot's own number, its parent, its flag word (`0x1F` on the head, `0x7` on the legs, belly, chest and arms, `0x3` on the pelvis, feet, shoulders and hands) and its child slots.

#### Scenario: Head and chest
- **WHEN** the chest and head slots are read
- **THEN** the chest's parent is the belly, its children are the head and both shoulders, and the head's flag word is `0x1F`

### Requirement: Coverage pass
The system SHALL compute each worn garment's cover and visible mask from its occupancy bytes and the worn set's coverage union, walking its layers from the outermost in: a union byte above a layer joins the cover where the garment holds nothing higher, `0x80` hides the slot unless the garment's higher bytes meet the slot's mask, `0x40` hides the slot's partial bit, and a garment holding none of a mask hides it too.

#### Scenario: A helm over a hood
- **WHEN** a hood with head byte `0x03` on layer 1 is worn under a helm with head byte `0x83` on layer 2
- **THEN** the hood's head cover is `0x83`, its head and head partial bits are hidden, and its head node is not chosen

### Requirement: Layer score
The system SHALL score a garment node against its slot's cover as sixteen plus its matched sub-part bits, eight off for each unmatched bit, reading the node's layer bits from its name's third and fourth characters without a digit check.

#### Scenario: Breastplate layers
- **WHEN** nodes of layer bits `06` and `02` are scored against a cover of `0x06`
- **THEN** they score 18 and 17

### Requirement: Slot assembly
The system SHALL keep a slot's best-scoring groups in the builder's stack order (the last child first), prune each child slot the garment has a word in under each group's parent-slot node (the best child, ties to the first, else the same-bits fallback), drop groups below the best total by swapping in the last, and take the first group's nodes where the slot's select mask meets the garment's visible mask.

#### Scenario: Chest and shoulder
- **WHEN** a garment offers chest nodes of layer bits `02` and `06` against a chest cover of `0x06`, the `06` chest having a shoulder node of bits `06` against a shoulder cover of `0x06`
- **THEN** the `06` chest and its shoulder are chosen, in one group of score 36

### Requirement: Same-bits fallback
The system SHALL search a garment's trees, when a group's parent-slot node has no child in the slot, for parent-slot nodes of the same layer bits and take their best child in the slot, ending a tree's search at the first parent-slot node of other bits.

#### Scenario: A mismatching forearm first
- **WHEN** the first forearm node met has other layer bits than the wanted ones
- **THEN** that tree yields no hand node

### Requirement: Body mask and cull bits
The system SHALL hide a body part when the worn set's coverage union has `0x80` in the slot's innermost byte (and its partial bits for `0x40`), and give a slot's merged triangles cull bit `0x400000` when its second cull mask misses the visible mask, else `0x200000` when its first does, else none.

#### Scenario: A hide bit on an outer layer
- **WHEN** a slot's union word has `0x80` only in its fourth byte
- **THEN** the slot's body part is not hidden

### Requirement: Normal layers
The system SHALL blend the race's physique normal map `i + 1` by the morph controller's weight `i` when it is over 0.01 in double precision, add the feral level to the first, show the extra map (the men's `nrmm5` for both sexes) for its weight or the feral level, and build no layer without a controller or for a woman of a race with only the men's set.

#### Scenario: Weights to maps
- **WHEN** a man's weights are 0.5, 0.02, 0.01 and 1.0 with 0.03 on the extra
- **THEN** maps 1, 2 and 4 and the extra map blend, at 0.5, 0.02, 1.0 and 0.03

### Requirement: Skin job record fields
The system SHALL replace a zero look seed before drawing the two skin-map offsets, colour a bearded man's beard with the hair picker's diffuse halfway to grey 0.3, and on a reset reread the pixel format and keep the job's maps.

#### Scenario: A zero seed
- **WHEN** a record's look seed is zero
- **THEN** a fresh seed is drawn, kept for the record, and the offsets come from it

### Requirement: Hair
The system SHALL grow a style's hair only for a race with hair whose worn set's head coverage union lacks bit 1, show the `?` roots only while that union has bit 0 and the others only while it does not, and tint it with the hair material pass's colour worked in its mixed precision.

#### Scenario: Mixed-precision colour
- **WHEN** the record's hair bytes are 0, 13 and 110 at feral 0
- **THEN** the diffuse is `0x3e37d4fe 0x3e11aa7d 0x3e008f99` and the specular `0x3f466f8e 0x3f288958 0x3f18abb4`

### Requirement: Hair rebuild
The system SHALL decide a look rebuild's hair by comparing the built hair code (none, `?` roots, full) with the wanted one: keep when equal, remove when none is wanted, rebuild otherwise; the style number plays no part.

#### Scenario: A hood put on
- **WHEN** full hair is built and the head union gains bit 0
- **THEN** the hair is rebuilt with its `?` roots

### Requirement: Equip pass
The system SHALL take a record's first two hands as the equip pass does: the off hand's item takes the torch path when it has the torch section, else a weapon section makes it a weapon before a shield section makes it a shield, an item with neither is not equipped, and each hand's item is resolved by its own index.

#### Scenario: Two equal swords
- **WHEN** a placed character holds the same sword item in both hands
- **THEN** each hand is resolved from its own slot

### Requirement: Body passes
The system SHALL reproduce the generator's body passes on its node tree: held marks on the root's direct mesh children, the slot chain from the root's slot 1, re-framing on the base body's slot world frames, the frame save and its rows-only restore, the state copy by slot, and the dynamics build, split and rebuild.

#### Scenario: Held marks
- **WHEN** a held item's root is marked
- **THEN** only its direct mesh children take the held flag

### Requirement: Merge order and poses
The system SHALL merge a garment's chosen nodes along the slot tree and restore a saved bone tree by matching each record to its parent bone's nearest child within 20 cm, skipping a record's subtree when none matches.

#### Scenario: Merge order
- **WHEN** every slot has a chosen node
- **THEN** they merge in the order 1 to 13, 17, 14 to 16, 18

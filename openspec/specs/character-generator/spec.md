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
The system SHALL visit a garment's chosen nodes for the skin weights along the slot tree, append the dressed mesh's geometry in slot order 1 to 18, and restore a saved bone tree by matching each record to its parent bone's child whose rest local offset is nearest the record's origin in the parent's restored frame, only under 20 cm (the first nearest kept), skipping a record's subtree when none matches. A matched child SHALL take the record as its world frame and re-derive only its local rows.

#### Scenario: Merge order
- **WHEN** every slot has a chosen node
- **THEN** the weight pass visits them in the order 1 to 13, 17, 14 to 16, 18

#### Scenario: Append order
- **WHEN** a garment has chosen nodes in slots 13, 14, 17 and 18 and every slot is visible
- **THEN** they are appended in the order 13, 14, 17, 18

#### Scenario: Twenty centimetres
- **WHEN** a child record lies 19.99 cm, then 20.01 cm, from its parent's child rest offset
- **THEN** the first is placed on that child and the second is skipped with its subtree

#### Scenario: Siblings near one child
- **WHEN** two sibling records lie 15 cm either side of one child's rest offset
- **THEN** both match it, and the child keeps the second record's frame and its rest offset

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

### Requirement: Flat saved poses
A flat saved pose SHALL map slots 1 to 18 by the saved part table (pelvis, belly, chest, right arm, head, left arm, legs) and ignore later slots; each part SHALL take its slot's world frame and re-derive only its local rows.

#### Scenario: Flat slots
- **WHEN** the 13 shipped `exanima05_sw` flat poses are mapped by the slot table
- **THEN** each body's pelvis–belly, shoulder and thigh distances are rigid

### Requirement: Running merged character mesh
The game SHALL draw each dressed character whose body model numbers at least fifteen slots as one skinned mesh merged from the dressed-mesh plan, with one sub-mesh per material run, and SHALL hide the body nodes it replaces and the body slots a garment covers. Node frames SHALL be composed in the file's row-vector convention, so a child one metre along x under a parent turned a quarter about z sits one metre along y.

#### Scenario: Frames
- **WHEN** a transform is turned into a file frame and composed under a parent turned a quarter about z
- **THEN** the round trip gives the same transform and the child's origin and axes match the Bevy hierarchy's

#### Scenario: Runs
- **WHEN** two built parts with different materials are merged and sliced
- **THEN** there are two runs of three vertices each, with rebased indices, their own joints and tangents

### Requirement: Running skin composite
The merged skin run SHALL take the character's composited colour, surface and normal maps, generated once per distinct look (stem, tone, side, layers and beard; not the look seed) and read from the race maps at the target's own coordinates. Until the composite is ready, the run SHALL draw the race maps plus tone.

#### Scenario: Cache key
- **WHEN** two jobs differ only in tone, side, a layer amount or a beard
- **THEN** their keys differ, and an equal job repeats its key

### Requirement: Twenty-word layer level
The apparel cache SHALL order garments by their layer level, the highest layer byte over all twenty occupancy words (bones 1 to 20), inserting each after every entry of a level at most its own.

#### Scenario: Bone twenty
- **WHEN** a garment whose only layer-3 byte lies on bone 20 is worn before a layer-0 garment
- **THEN** the layer-0 garment is planned first

### Requirement: Slot-walk stack overflow
The slot assembler SHALL log a body-part stack overflow when its walk stack holds more than 31 entries and SHALL continue the walk.

#### Scenario: Forty children
- **WHEN** a slot's node sits among forty children of one node
- **THEN** the slot still chooses it

### Requirement: Build clamps
The build SHALL read the record's feral level and age within 0 to 1 (a NaN kept) wherever the look, darkening and stats use them.

#### Scenario: Over one
- **WHEN** a record's feral level is 1.5 on a race whose skin darkens with age
- **THEN** the darkening is 1.0

### Requirement: Worn special kinds
The character build SHALL dispatch each worn item's special kind in wearing order: kind 2 grants the second wind, kinds 3, 4 and 6 register their item effect with the item's crystal flag and charge bytes, kind 5 wards the mind, and kind 7 sets the focus regeneration bonus to the float with bits 0x38D28EB4; kinds 0 and 1 do nothing here. This SHALL hold for every item of the shipped object database, which carries kinds 2 to 7.

#### Scenario: Mixed outfit
- **WHEN** a body wears, in order, kinds 1, 3, 4, 6, 3, 7 and 7
- **THEN** the registered effects are the two healing items, the suit and the gauntlets in that order, there is no second wind or ward, and the bonus has bits 0x38D28EB4

#### Scenario: Ward and wind
- **WHEN** a body wears a kind 5 and a kind 2 item
- **THEN** it is warded and has the second wind, with no focus bonus

### Requirement: Mind ward in play
A spawned body wearing a mind ward SHALL be perceived as hidden by other characters, and its caster SHALL refuse every power it casts; a focus charm SHALL set its caster's focus regeneration bonus. Without them the refusal bit and the bonus SHALL be cleared.

#### Scenario: Caster
- **WHEN** a caster is given a ward and a charm
- **THEN** its refusing bit is set and its bonus has bits 0x38D28EB4, and both clear when it is given neither

### Requirement: Hair cloth
With the hair cloth switch on (the default; `EXA_HAIR_CLOTH=0` turns it off), the character build SHALL hang each cloth-marked node of a hair style as cloth, pinned by its faces' flags, giving each style root the hair's cloth record and leaving a marked node below a root with the built-in record. With the switch off the styles stay rigid on the head.

#### Scenario: A long style
- **WHEN** a character with the first long style (`hair hula01`) is built with the switch on
- **THEN** its cloth-marked parts become particle-mode cloths with some vertices pinned and the rest free

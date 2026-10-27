# apparel Specification

## Purpose
Fits the original apparel and armour models to animated bodies so characters
can be dressed and equipped.

## Requirements

### Requirement: Fitting by bone index
The system SHALL attach each apparel node to the body part whose bone index matches the first two digits of the node's name, so the piece follows that part.

#### Scenario: Doublet on a walking body
- **WHEN** `exarust character --outfit doublet_b Default/Walk` runs
- **THEN** the doublet's chest, belly and shoulder pieces move with the body's chest, belly and shoulders

### Requirement: Run-time texture sets
The system SHALL resolve a material named `&<model><set><n>` to the texture set named by the model base padded with underscores to eight characters, the model variant and the set letter.

#### Scenario: Doublet texture
- **WHEN** a piece uses material `&doublet_ba1`
- **THEN** its diffuse texture is `doublet_bad` and its normal map `doublet_ban`

### Requirement: Painted apparel
The system SHALL colour worn apparel by painting each item's texture set from its shade map, layer masks and the appearance (material and palette colour per layer, quality, damage, dirt) of the worn item, or, for apparel given only by its model, of the first object-database item worn as that model. `exa-tool paint` SHALL choose its item the same way.

#### Scenario: Doublet
- **WHEN** a character wears `doublet_a`
- **THEN** its run-time materials show the cloth colours of the item "Doublet A" instead of the grey shade map

#### Scenario: Missing appearance
- **WHEN** no item is worn as a model
- **THEN** the model keeps its unpainted material

#### Scenario: Underwear
- **WHEN** a character wears the underwear
- **THEN** the braies are painted as rough cloth in the palette's pale grey on both masks of their texture set, at the underwear item's quality and damage and without dirt, not in the dyed silk of the item "Braies_P"

#### Scenario: Painting the braies from the command line
- **WHEN** `exa-tool paint braies_a braies__aa out.png` runs
- **THEN** it names the item "Braies_A" and writes pale grey rough cloth

### Requirement: Body coverage
The system SHALL hide the body parts that worn apparel covers, as given by each item's per-slot flags.

#### Scenario: Gambeson
- **WHEN** a character wears a gambeson, trousers and boots
- **THEN** no skin shows through them on the torso, arms, legs or feet

### Requirement: Worn models
The system SHALL dress a character in the model its item's body names, which is the model the item is worn as, and SHALL NOT use for that the model the item shows on the ground. An item without a ground model SHALL still be worn. This SHALL hold for every apparel item of `objdb.rdb` and of every level's own object database.

#### Scenario: Quilted trousers
- **WHEN** a character wears "Trouser Qui A", whose ground model is `Trouser_At`
- **THEN** it wears the `trouser_a` model of `Characters.rpk`

#### Scenario: Item without a ground model
- **WHEN** a character wears the underwear item "Braies_A"
- **THEN** it wears the `braies_a` model

### Requirement: Underwear
The system SHALL keep every human it makes at run time (the player and the character view's body) in the original's underwear: when nothing such a character wears covers the groin on the innermost layer, it SHALL also wear the object database's underwear item (braies), beneath everything else. Characters of other races, and characters a level places, SHALL NOT be given underwear.

#### Scenario: No outfit
- **WHEN** `exarust character` runs without `--outfit`
- **THEN** the body wears pale grey linen braies over the hips and thighs and nothing else

#### Scenario: Shirt
- **WHEN** `exarust character --outfit shirt_b` runs
- **THEN** the body wears the shirt with the braies beneath it

#### Scenario: Skeleton
- **WHEN** `exarust character --race 1` runs
- **THEN** the skeleton wears nothing

### Requirement: Groin cover
The system SHALL count trousers, hose and braies as covering the groin on the innermost layer, and shirts, tunics, coats, mail legs and plate as not covering it. A character wearing something that covers the groin SHALL NOT also wear the underwear, and the underwear SHALL count as its own cover.

#### Scenario: Mail legs
- **WHEN** `exarust character --outfit trousers_c` runs
- **THEN** the body wears the braies under the mail legs and no bare groin shows

#### Scenario: Trousers
- **WHEN** `exarust character --outfit trouser_a` runs
- **THEN** the body wears the trousers and no braies

#### Scenario: Braies given
- **WHEN** `exarust character --outfit braies_a` runs
- **THEN** the body wears one pair of pale grey braies

### Requirement: Painted colour space
The system SHALL read painted apparel colour in the original's colour space.
- The finishes SHALL read the shade map, layer masks and library textures as stored, without sRGB decoding, and SHALL take the palette colour as computed.
- The painted colour SHALL keep the finishes' values as sRGB texels, decoded to linear light when sampled for lighting.
- Its mip levels SHALL average colour in linear light.
- The painted surface and normal maps SHALL be read as linear data.

#### Scenario: Pale grey on a mid-grey shade map
- **WHEN** palette colour 1 is painted by a finish without its own painter over a mid-grey shade map (128)
- **THEN** the texel is 160 and the surface is lit with 0.3515

#### Scenario: Rough cloth
- **WHEN** rough cloth in palette colour 1, at quality 160 and damage 64, is painted over a mid-grey shade map and mid-grey library textures
- **THEN** the texel is 96 and the surface is lit with 0.117, as the original's cloth program gives

#### Scenario: Seen from a distance
- **WHEN** a painted colour map's four texels are one white and three black
- **THEN** their mip texel is 137 (linear 0.25), not 64

### Requirement: Painted layers
The system SHALL paint an item's appearance layers in order, all eight of them, skipping a layer without a mask and ending the list at the first layer whose mask the texture set lacks.

#### Scenario: Eight masks
- **WHEN** an appearance names masks 1 to 8
- **THEN** all eight layers are painted

#### Scenario: Missing mask
- **WHEN** the braies' item lists four masks and their texture set has two
- **THEN** only the layers before the first missing mask are painted

### Requirement: Procedural material programs
The system SHALL paint each procedural material with the original's programs for its family: material-scale textures repeating by the material's first colour texture, damage-scale ones once per 1024 texels, each layer's colour and wear weight, and the specular pass measuring its creases on the painted normal map after every layer's normal pass. Plate materials SHALL read their own rust grain map.

#### Scenario: Pale linen
- **WHEN** rough cloth in palette colour 1 at quality 160 and damage 64 is painted over a mid-grey shade map and mid-grey library textures
- **THEN** its colour texel is 96

#### Scenario: Plate grain
- **WHEN** material 13 is painted
- **THEN** its colour, specular and normal passes read `MtlRustA04G`

### Requirement: Sees in the dark
An item body whose special kind byte (at 0x214 of the body) is 1 SHALL be read as seeing in the dark, and a body too short to hold the byte SHALL read kind 0. In the installed object database exactly the three night-vision helms SHALL see in the dark.

#### Scenario: The byte and the corpus
- **WHEN** a body carries kind 1 at 0x214, a body ends before it, and the installed `objdb.rdb` is scanned
- **THEN** the first reads 1, the second 0, and the items of kind 1 are "UCA Helm A", "UGA Helm FB" and "UW Helm NV"

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

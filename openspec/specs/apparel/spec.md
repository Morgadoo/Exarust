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

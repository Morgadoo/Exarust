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
The system SHALL colour worn apparel by painting each item's texture set from its shade map, layer masks and the appearance (material and palette colour per layer, quality, damage, dirt) of the object-database item that shows the model.

#### Scenario: Doublet
- **WHEN** a character wears `doublet_a`
- **THEN** its run-time materials show the cloth colours of the item "Doublet A" instead of the grey shade map

#### Scenario: Missing appearance
- **WHEN** no item shows a worn model
- **THEN** the model keeps its unpainted material

### Requirement: Body coverage
The system SHALL hide the body parts that worn apparel covers, as given by each item's per-slot flags.

#### Scenario: Gambeson
- **WHEN** a character wears a gambeson, trousers and boots
- **THEN** no skin shows through them on the torso, arms, legs or feet

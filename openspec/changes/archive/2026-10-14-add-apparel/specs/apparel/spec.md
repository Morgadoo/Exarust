# Spec Delta

## Purpose

Fits the original apparel and armour models to animated bodies so characters
can be dressed and equipped.

## ADDED Requirements

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

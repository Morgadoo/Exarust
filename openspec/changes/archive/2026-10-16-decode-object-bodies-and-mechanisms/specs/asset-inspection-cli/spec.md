## ADDED Requirements

### Requirement: Body and mechanism inspection
`exa-tool bodies` SHALL print the rigid bodies and cloth markers of a chunk-tree entry's nodes (root or child, class, mobility, rest points, spheres, links with a slider's travel, sound slots, in cm); `exa-tool mechanisms` SHALL print the door and operative records of `objdb.rdb` or of a level's own database with the mechanism each gives its model; `exa-tool corpus bodies` SHALL count bodies, links, sliders and cloth markers per model pack.

#### Scenario: A lever
- **WHEN** `exa-tool bodies Objects "lever a01"` runs
- **THEN** one root body is printed with two axle pins and an arm link of 33.81 cm slack

#### Scenario: First level
- **WHEN** `exa-tool mechanisms exanima01` runs
- **THEN** 43 doors and 72 operative records are printed

#### Scenario: Corpus
- **WHEN** `exa-tool corpus bodies` runs
- **THEN** `Objects.rpk` shows 1 163 of 1 253 models with bodies, none undecodable, 69 with links, 65 hung from the world

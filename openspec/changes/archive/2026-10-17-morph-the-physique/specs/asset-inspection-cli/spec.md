## ADDED Requirements

### Requirement: Morph set inspection
`exa-tool morphs` SHALL print every morph set of the installed `Resource.rpk`: the file and node carrying it, the node's place, the landmarks by bone and how far each target moves the positions and the joints, in cm. With `--records` it SHALL print the weights every placed character's record gives its race's set, then how many were placed, how many have a set, how many weigh nothing, and each weight's range.

#### Scenario: Retail installation
- **WHEN** `exa-tool morphs` runs against a retail installation
- **THEN** it lists `ancientmorph.rfc` and `humanmorph.rfc`, each with 18 landmarks, 2286 positions and 11 targets on `/00Male_Ba` at (0, 102.98, 0) cm, target 1 reaching 11.61 cm and target 10 13.54 cm

#### Scenario: Placed records
- **WHEN** `exa-tool morphs --records` runs against a retail installation
- **THEN** it counts 477 placed characters, 126 with a morph set and none weighing nothing

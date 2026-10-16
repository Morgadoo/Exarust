## ADDED Requirements

### Requirement: Debug wounds
`--debug-wound bone,radius_cm[,dx,dy,dz]`, repeatable, SHALL paint a test wound on the player's body or the character view's body once it is dressed and bound: at its body part's front (the part's visible vertex furthest along the body's facing), of that radius, smeared along d (cm, in the body's frame, +Y up, +Z forward). A bone SHALL be given by number (1–18) or by part name (`chest`, `luarm`, …).

#### Scenario: Chest and arm
- **WHEN** `exarust --debug-wound chest,20 --debug-wound luarm,12 character` runs
- **THEN** dark red blood with speckled edges and a wet sheen covers the front of the chest and of the left upper arm

#### Scenario: Smeared down the belly
- **WHEN** `exarust --debug-wound belly,15,0,-40,0 character` runs
- **THEN** the blood runs from the belly down over the braies

#### Scenario: Bad value
- **WHEN** `--debug-wound chest` is given without a radius
- **THEN** exarust prints the expected form and exits with code 2

## ADDED Requirements

### Requirement: Character creation scene
`exarust character` SHALL show the body in the original's character-creation scene: the `sgccflr01` floor and its two omni lights, with no sun and no ambient. Every surface SHALL fade to black by 2 m horizontal distance from the origin, over a black background. The camera SHALL be the original's: target (0, 0.6, 0) m, 4 m along (−2, 2.1, 3.6), 50° vertical field of view. A "Torch" or "1H & Torch" action SHALL show the body holding the lit torch in the hand that moveset raises.

#### Scenario: Default view
- **WHEN** `exarust character Default/Idle` runs
- **THEN** the body stands on cobblestones in a pool of white and warm light with black all around

#### Scenario: Torch moveset
- **WHEN** `exarust character Torch/Idle` runs
- **THEN** the body holds the burning torch raised in its right hand

#### Scenario: Torch beside a weapon
- **WHEN** `exarust character "1H & Torch/Idle"` runs
- **THEN** the body holds the burning torch raised in its left hand

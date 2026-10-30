## ADDED Requirements

### Requirement: Weapon blood list in the game
The running game SHALL keep one weapons' blood list. A weapon's first bleeding wound registers it (a clean spot, with its tip's length), each bleeding wound adds half its R to its spot, and the weapon's drawn blood is its spot's amount. A weapon that leaves the world SHALL release its spot for the next weapon.

#### Scenario: Two weapons
- **WHEN** one weapon deals two wounds of R 0.4 and another one of R 0.2
- **THEN** the first holds spot 1 with 0.4 and the second spot 2 with 0.1

### Requirement: Weapon blood at a world reset
The original's world reset (a new session, a save load, an arena round's start or end, a scripted jump, a quit) SHALL drop every spot of the list, and every weapon that survives it SHALL hold no blood and no spot, as the bodies the original rebuilds do. A weapon whose blood no longer shows SHALL lose its stain pass, and draw a new one with new blood. A door's level change SHALL keep the list, the weapon's spot and its stain.

#### Scenario: Reset
- **WHEN** the player's weapon holds 1 of blood and the world is reset behind a level change
- **THEN** the list is empty, the weapon holds 0 and its stain is gone; its next bleeding wound registers it in spot 1

#### Scenario: Door
- **WHEN** the player carries the same weapon through a door's level change
- **THEN** it keeps its blood, its spot and its stain

## ADDED Requirements

### Requirement: Target pass picks the foe
When the target pass enters combat, the brain's foe SHALL become the character the pass picked, whether or not it is the player; a pick that is already the foe SHALL leave the foe as it is. A body whose AI clock is held SHALL enter no combat until the hold passes.

#### Scenario: A neutral takes the fighter
- **WHEN** a neutral brain rates a fighting brain beside it an engagement
- **THEN** it enters combat with that brain as its foe

#### Scenario: A held brain waits
- **WHEN** a hostile brain's AI clock is held
- **THEN** it does not engage until the hold is cleared

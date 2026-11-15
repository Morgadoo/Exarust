## ADDED Requirements

### Requirement: Second wind heal sound
A second wind that fires SHALL play the heal set `StmHeal01` at the fighter's place at volume 0.1 and pitch 1; a fighter no longer in the world SHALL sound nothing.

#### Scenario: A fired second wind
- **WHEN** the second wind fires for a fighter standing at (1, 2, 3)
- **THEN** one heal sound is asked for at (1, 2, 3)

### Requirement: Swing voice loop sources
A fighter's swing voice SHALL keep the body's pair of looping sources (its voice and its foley, on which the clothing rustle plays): reserved at the fighter's first swing, freed with its voice when it dies, and reserved again at its first swing after a revive; a dead fighter's swing SHALL reserve nothing.

#### Scenario: Death and revive
- **WHEN** a fighter swings, dies, swings dead and, revived, swings again
- **THEN** its first swing and its swing after the revive each reserve a pair and the others reserve none

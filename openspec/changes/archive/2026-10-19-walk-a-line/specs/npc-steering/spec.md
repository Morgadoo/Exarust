# Spec Delta

## ADDED Requirements

### Requirement: Line replan
In the scene, a blocked line from the ground to the carrot SHALL clear the run bit and ask for a re-plan, and SHALL leave the carrot. A clear line SHALL leave the run bit and the re-plan flag. Out of the scene, and a higher scene bit alone, SHALL not walk.

#### Scenario: A wall asks for a re-plan
- **WHEN** an open line is walked in the scene with the run bit set, the same line with a floor of 40 at cell 4 is walked in the scene, out of the scene, and with only a higher scene bit while a re-plan was already asked
- **THEN** the open line keeps the run bit and does not re-plan, the walled line clears the run bit, asks for a re-plan, and keeps the carrot, the out-of-scene line keeps the run bit and does not re-plan, and the higher bit keeps the run bit and the re-plan that was already asked

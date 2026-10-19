## ADDED Requirements

### Requirement: Skill selector show
The show SHALL clear bits 0x60 on the object when one exists, apply a positive selection, or apply −1 when the word's low 16 bits are nine or ten. It SHALL apply nothing for any other word or a missing object, and the base method SHALL always run.

#### Scenario: The selection
- **WHEN** an object exists and the selection is 3
- **THEN** 3 is applied and bits 0x60 clear
- **WHEN** the selection is 0 and the word is 9 or 10
- **THEN** −1 is applied
- **WHEN** the selection is 0 and the word is 4
- **THEN** nothing is applied
- **WHEN** no object exists
- **THEN** nothing is applied and the base still runs

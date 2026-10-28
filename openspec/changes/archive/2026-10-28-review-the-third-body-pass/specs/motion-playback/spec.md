## ADDED Requirements

### Requirement: Ease arithmetic
The smooth ease SHALL be (3 − 2f)·(f·f) in single precision. The fast ease SHALL be (f·0.11 + 1) − 1/(8f + 1), the first term and the sum in double with the double 0.11, the reciprocal single, narrowed once.

#### Scenario: Ends of the fast ease
- **WHEN** the fast ease is taken at 0 and at 1
- **THEN** it gives 0 and 0.99889

### Requirement: Hold bones
The hold pose and the plain capture SHALL keep the last targets of the same bones, skin bones 1 to 7 (the pelvis and legs), and take the physical turn for the rest.

#### Scenario: The last leg bone
- **WHEN** a hold pose is taken from a body whose seventh bone has a kept target
- **THEN** the seventh bone keeps it and the eighth takes its physical turn

## ADDED Requirements

### Requirement: Creation appearance settle
Settling SHALL compute `(0.5 − source) × 2`. A distance from the target under the epsilon `0x33ffffd7` SHALL store nothing. Otherwise the value SHALL be stored; a last stamp plus 2 above the clock SHALL set the pending flag, and otherwise the stamp SHALL take the clock and the callback SHALL run.

#### Scenario: Settle, pend and fire
- **WHEN** the source is 0.25 and the target is 0.5
- **THEN** nothing is stored
- **WHEN** the target is 0, the last stamp 100 and the clock 101
- **THEN** 0.5 is stored and the pending flag is set
- **WHEN** the clock is 102
- **THEN** 0.5 is stored, the stamp becomes 102 and the callback runs

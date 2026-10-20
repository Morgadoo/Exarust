## ADDED Requirements

### Requirement: Spirit form charge
The spirit form SHALL charge at one value per 30 s from state 3, clamp at 1 and become held in state 1; held SHALL keep the value at 1.

#### Scenario: Charging to full
- **WHEN** a form at 0 charges for 15 s
- **THEN** the value is 0.5 and the state is still charging
- **WHEN** it reaches 1
- **THEN** the state is held and the value stays at 1

### Requirement: Spirit form fade
A fade SHALL lower the value by one per 30 s and end at 0 in state 0 once it falls below 0.002.

#### Scenario: Fading out
- **WHEN** a full form fades for 15 s
- **THEN** the value is 0.5 and the strength 500
- **WHEN** it fades for 30 s
- **THEN** the form is inactive with value 0

### Requirement: Spirit form publish
While fading the strength SHALL be 1000·value. Otherwise the smooth value SHALL be 1 − (1 − value)², the strength (1 − smooth)·rate + 1000·smooth, and the vector scale 1 − value.

#### Scenario: Half charged
- **WHEN** a form at 0.5 charges with rate 100
- **THEN** the smooth value is 0.75, the strength 775 and the vector scale 0.5

## ADDED Requirements

### Requirement: Hit veto
The damage chain SHALL ask the struck body's veto, when it carries one, about every weapon contact on a living limb that lands short of the weapon's tip (a not-a-number share included), after the regeneration ramp's reset; a veto that answers yes SHALL end the contact's handling with nothing dealt and no striker reaction, and SHALL announce the pass.

#### Scenario: A phased blow
- **WHEN** a sword strikes a fighter whose veto passes the blow
- **THEN** the fighter keeps its health, no shove is made and the pass is announced

### Requirement: Phasing streak
A phasing veto SHALL pass a blow when the engine generator's percentage is at most `100 >> (s + 1)` for streak `s ≥ 0` or `100 − (100 >> (1 − s))` for `s < 0`, raising a non-negative streak and restarting a negative one at 0; a refusal SHALL lower a non-positive streak and restart a positive one at 0. A pass SHALL draw the sound's pitch `0.9 + 0.2·r` in double.

#### Scenario: Streak after a pass
- **WHEN** a fresh streak passes a blow
- **THEN** the streak is 1 and the next blow passes only at a percentage of 25 or less

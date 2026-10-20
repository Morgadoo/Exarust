## ADDED Requirements

### Requirement: Skill screen fade-in
With bit 1 clear nothing SHALL change. With the deadline at or below the clock the state SHALL become 4; page 2 SHALL return alpha 0 and any other page SHALL close. Otherwise the sound SHALL become `(deadline − clock) × sound / delay`.

#### Scenario: Clear, closing and fading
- **WHEN** bit 1 is clear
- **THEN** nothing changes
- **WHEN** bit 1 is set, the deadline 500 and the clock 600 on page 2
- **THEN** the state is 4 and alpha is 0
- **WHEN** the page is 1
- **THEN** the close runs
- **WHEN** the deadline is 1000, the clock 800, the sound 0.5 and the delay 250
- **THEN** the sound becomes 0.4

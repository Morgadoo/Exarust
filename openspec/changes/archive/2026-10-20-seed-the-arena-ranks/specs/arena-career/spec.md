## ADDED Requirements

### Requirement: MT19937 stream
The system SHALL provide the standard MT19937 generator and a unit draw of `next_u32 · 2⁻³²`.

#### Scenario: Known vector
- **WHEN** the generator is seeded with 5489
- **THEN** its first five outputs are 3499211612, 581869302, 3890346734, 3586334585 and 545404204

### Requirement: Arena rank stream
The system SHALL advance the MT stream only for a rank roll, not for flag-2 or unranked templates.

#### Scenario: Prerolled and rolled
- **WHEN** Challenger rolls at slot 0
- **THEN** the MT stream is untouched
- **WHEN** Elimination rolls at slot 3
- **THEN** exactly one unit draw is consumed

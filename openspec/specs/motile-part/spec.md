# motile-part Specification

## Purpose
The character body part's state, impact, voice and wake rules as the original computes them: the strike window and mark, the part hit sound, the voice request, the wake and the attack reach.

## Requirements

### Requirement: Strike window and mark
A part's contact SHALL count as a strike only when both the strike window and the chain bit are set. Marking a chain SHALL set the window, the chain bit and the walk's separate mark bit; clearing SHALL drop all three, and an attack interrupt SHALL drop the chain bit.

#### Scenario: Mark and clear
- **WHEN** a part is marked as a striker and then cleared
- **THEN** it strikes after the mark with the mark bit set, and after the clear the window, chain bit and mark bit are all clear

### Requirement: Part hit sound
A part's hit sound SHALL skip an armour layer only when its coverage is below 0.5, so a NaN coverage counts as covering.

#### Scenario: NaN coverage
- **WHEN** a layer of NaN coverage is the first to cover
- **THEN** the layer's material class sound is chosen

### Requirement: Voice request
The voice request SHALL apply its two volume gains in the original's order and SHALL compute the scream wait as an unwrapped 64-bit sum.

#### Scenario: Late clock
- **WHEN** the clock plus the scream wait exceeds 32 bits
- **THEN** the wait does not wrap

### Requirement: Wake
The wake tone and readiness SHALL be summed in double precision, and the grace sums SHALL be unwrapped 64-bit sums. Waking from a knock-out SHALL notify the caller and SHALL set stance 0 in the knocked branch.

#### Scenario: Grace past 32 bits
- **WHEN** the knock-out clock plus the grace exceeds 32 bits
- **THEN** the character stays down until the unwrapped sum is reached

### Requirement: Attack reach
A character's attack reach SHALL be size × ((1 − 0.3·age) × base) in double precision with base = (1 − 0.2·encumbrance) × 80 and size the square root of the scale; a negative age SHALL use size × base, and a NaN age SHALL give a NaN reach.

#### Scenario: Human at age 0.5
- **WHEN** a human of scale 1 and no encumbrance has age 0.5
- **THEN** the reach is 1 × ((1 − 0.15) × 80) = 68 cm

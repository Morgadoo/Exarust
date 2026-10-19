# Spec Delta

## ADDED Requirements

### Requirement: A deep wound is noticed
A lasting wound SHALL be noticed when that wound times 0.75, widened, times 0.2, is greater than 0.25 widened times 0.05. Bits 0x3daaaaab are not noticed. The next float is. A non-number is not. The hit path records the notice for a chest cut.

#### Scenario: The next float after the line is noticed
- **WHEN** a lasting wound of bits 0x3daaaaab is weighed, the next float is weighed, a non-number is weighed, and a chest cut is dealt through the hit path
- **THEN** the first is not noticed, the next float is, the non-number is not, and the chest cut records one notice for that fighter

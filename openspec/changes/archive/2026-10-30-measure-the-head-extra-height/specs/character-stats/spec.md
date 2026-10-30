# Spec Delta

## ADDED Requirements

### Requirement: Head extra height
The system SHALL measure a character's head extra height as the original's armour pass does: the largest display byte among the worn pieces whose body carries the head mark (its eleventh per-slot entry is non-zero), whatever the piece's coverage of the eleventh bone, plus (physique − 0.5) × 8 in single precision.

#### Scenario: Thickest marked piece
- **WHEN** a character of physique 0.5 wears marked pieces with displays 60 and 40
- **THEN** its head extra height is 60, and 64 at physique 1

#### Scenario: Unmarked or off the head
- **WHEN** the only piece worn has display 60 and no head mark
- **THEN** the head extra height is 0; with the mark but covering only other bones it is 60

#### Scenario: Shipped pieces
- **WHEN** the installed item database is read
- **THEN** 95 of its 380 worn pieces carry the head mark, the thickest marked display is 10, and one marked piece covering no part of the head counts with display 2

### Requirement: Coverage cap keeps NaN
A worn piece's coverage share SHALL be capped at 1 by an ordered compare, so a share that is NaN stays NaN.

#### Scenario: Cap
- **WHEN** shares 1.5, 0.5 and NaN are capped
- **THEN** they become 1, 0.5 and NaN

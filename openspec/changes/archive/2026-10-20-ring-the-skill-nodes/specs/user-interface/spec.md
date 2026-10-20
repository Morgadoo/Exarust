## ADDED Requirements

### Requirement: Skill ring position
The angle SHALL be `(slot − 3.5) × 2π / count + 1 − turn`. Each axis SHALL be the centre plus the product of the two offsets, the unit and the cosine (x) or sine (y), rounded to the nearest even integer and multiplied by the turn. The centre x SHALL lose one unit when scrolled with a positive page; the centre y SHALL lose three units on page 0.

#### Scenario: A node on the ring
- **WHEN** the count is 8, the offsets are 10 and 5, the turn is 1, the size is 200 by 100, the page is 0 and the slot is 3
- **THEN** the position is 285, −39
- **WHEN** the page is 1 and the screen is scrolled
- **THEN** the position is 281, −27

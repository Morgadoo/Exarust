## MODIFIED Requirements

### Requirement: Weapon condition
A copy's condition factor SHALL be (1 − d·0.4/255)·(q·0.5/255 + 0.8) in the original's mix of float and double, and the factory weapons' stat pass SHALL use the same factor. A weapon SHALL scale slash and pierce by it, crush by 0.7c + 0.3 and thrust by it, each capped at 160 after the byte wraps.

#### Scenario: A pristine sword
- **WHEN** a sword at quality 255 and no damage takes a template of slash 100 and crush 100
- **THEN** its slash is 130 and its crush 121

#### Scenario: A byte past 255
- **WHEN** a factory weapon at quality 255 and no damage has a pierce byte of 200
- **THEN** its pierce is 4, the low byte of 260, and the cap leaves it there

## ADDED Requirements

### Requirement: Count draws
A random table SHALL make its minimum plus a draw below its maximum less its minimum plus one count draws, at most its empty slots. The draw's span SHALL be taken as signed, so that a maximum below the minimum wraps as the original's does.

#### Scenario: An inverted count span
- **WHEN** a table of minimum 3 and maximum 1 with one count-only row fills five slots
- **THEN** it makes two count draws

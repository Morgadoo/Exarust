## MODIFIED Requirements

### Requirement: Domain wheel
The system SHALL place the domain wheel's six icons clockwise from the top of a ring of `round(76 · s)` pixels, Mind, Force, Body, Energy, Light and Displacement, with `s` the GUI scale (the interface unit times a quarter), and SHALL hit an icon when the squared whole-pixel distance to its rounded place is under `round((8 · unit · 0.65)²)` (433 at unit 4).

#### Scenario: The first icon
- **WHEN** the wheel's centre is (100, 100) at unit 4
- **THEN** icon 1 is at (100, 24) and icon 2 at (166, 62)

#### Scenario: The hit edge
- **WHEN** the mouse is 12 px right of and 17 px below icon 1 at unit 4
- **THEN** no icon is hit

#### Scenario: A larger unit
- **WHEN** the wheel's centre is (100, 100) at unit 8
- **THEN** icon 4 is at (100, 252), a mouse 41 px right of it hits it and one 42 px right does not

## ADDED Requirements

### Requirement: Domain wheel confirm gate
The wheel's confirm SHALL be refused while the parent tree's domain fade-out bit is set, and a confirmed choice SHALL add that bit to the tree's domain flags, keeping a running fade-in's bit.

#### Scenario: A click during the fade-in
- **WHEN** a known other domain is clicked while the tree's new domain fades in
- **THEN** the tree's domain flags hold both the fade-in and the fade-out bits and its domain deadline is 256 ms away

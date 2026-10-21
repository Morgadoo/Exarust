## ADDED Requirements

### Requirement: World links
The system SHALL pull a root body's part toward a point in a scene node by its world links, worked in centimetres in this order: a horizontal link, a slider, a spring, a plane link, then a plain link. A disabled link and a link past its last pass SHALL not run.

#### Scenario: A plain link with slack
- **WHEN** a link with a 10 cm slack and strength 1 is 20 cm from its anchor
- **THEN** it pulls its part 10 cm toward the anchor and records 20 cm as its length

#### Scenario: A plane link
- **WHEN** a plane link's anchor lies 10 cm along its axis
- **THEN** 80 % of the gap is removed, leaving 2 cm

#### Scenario: A late link
- **WHEN** a link whose last pass is 0 is run on pass 1
- **THEN** it does not move its part

### Requirement: World link kinds
A horizontal link SHALL step across the ground by twice the strength of the gap, capped at min(3·√mobility, 5) cm, less the point's approach. A slider SHALL clamp its fraction to the travel and have its correction applied twice. A spring SHALL pull a tenth of the strength less the approach and 0.4 of the point's motion.

#### Scenario: A slider
- **WHEN** a slider's correction is applied
- **THEN** the point moves by twice that correction

### Requirement: Plane and plain links
A plane link SHALL first remove 80 % of the gap along its axis. A plain link SHALL take the gap times the strength, at most 10·strength cm when it has no slack and beyond that its strength squared, or the excess over its slack once taut, forgetting its length inside the slack.

#### Scenario: A plain link without slack
- **WHEN** a link with strength 1 and no slack is 30 cm from its anchor
- **THEN** it pulls 10 cm

### Requirement: Soft welds
A soft-weld body link SHALL pull with a quarter of its strength times the gap less 0.4 of the two points' motion since the tick began, shared by 0.1/(m + m') of the mobilities rather than the link's own shares.

#### Scenario: A welded gap
- **WHEN** a soft weld of strength 1 joins points 0.1 m apart that have not moved
- **THEN** its correction is 0.025 m

### Requirement: Inverted tetrahedra are rebuilt
After the link iterations a part whose tetrahedron is inverted SHALL have its edges restored, its points set to those the tick began with shifted by how far its box centre has come, its body matrix built from them and the rest basis with each axis that leans against the cross of the other two (dot under 0.1, in the order y z, z x, x y) re-formed as that cross, and its particles put back on the rest tetrahedron under the matrix. Its previous points SHALL be left alone.

#### Scenario: A mirrored tetrahedron
- **WHEN** a part's last particle lies mirrored through the face of the other three
- **THEN** after the repair its particles lie on its rest tetrahedron

#### Scenario: A right-way tetrahedron
- **WHEN** the part's tetrahedron has its rest handedness
- **THEN** the repair leaves it unchanged

### Requirement: Slack reset
Resetting link lengths SHALL give each enabled body link that is neither a soft weld nor breakable a smoothed length equal to the slack when the gap's square exceeds the slack's, otherwise the gap, or 0 when the gap is under the epsilon.

#### Scenario: A gap inside the slack
- **WHEN** a link with slack 0.2 m is 0.1 m apart
- **THEN** its length becomes 0.1

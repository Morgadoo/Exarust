# Spec Delta

## MODIFIED Requirements

### Requirement: Picker point
A creation picker SHALL take a press over its field as `u = x / width` and `v = 1 − (1 − y / height)` over its client (the form less its border of 2 on every side, the point taken from the client's corner), worked in double precision and held to 0–1, ignore a point within an epsilon of the present one, and answer 7. Its marker, tone quads, hair cells and shade SHALL span the same client.

#### Scenario: Outside the field
- **WHEN** a 160-pixel tone field is pressed at (−100, 1000)
- **THEN** its point becomes (0, 1)

#### Scenario: Client corner
- **WHEN** a 160 × 160 tone picker is pressed 39 and 117 pixels from its client's corner
- **THEN** its point becomes (¼, ¾) over the 156 × 156 client

## ADDED Requirements

### Requirement: Compact skill pips
The compact skill screen SHALL draw a pip per 5 000 experience, each twelve units wide and overlapping the last by a unit; a last pip whose share is under 0.99 SHALL take `share · 0.8 + 0.1` of the width (rounded to even) and of the texture's u range, keeping the v range.

#### Scenario: Narrow last pip
- **WHEN** a skill's remainder narrows its last pip to half
- **THEN** the pip's right u is halfway along the u range and its v range stays 0.7 to 0.8

### Requirement: Compact skill node alpha
The compact skill screen SHALL scale its line and highlight alpha bytes by the screen's fade, rounded to even, and SHALL halve the scaled line byte for the technique in training.

#### Scenario: Faded training node
- **WHEN** the screen's fade is 0.31 and a row ends with the technique in training
- **THEN** a learned node's alpha is 37 and the training node's 18

# npc-combatant Specification

## Purpose
What every character behaviour keeps besides its own state: the references it forgets or remaps when a character dies or is swapped, the queued combat step and its clears, the nav footprint, and what a blow does to the struck character and its attacker.

## Requirements

### Requirement: References
Forgetting a character SHALL remove its contact, the last contact taking its slot, and clear the other target, the attention target, the body's target and the holder that name it. Replacing a character SHALL remap its first contact and every other reference. Reset SHALL drop the contacts, the attention target, the body's target and the holder, and keep the footprint, the alert and the other target.

#### Scenario: Three contacts
- **WHEN** contacts a, b, c are kept with a as target and attention target and b as the other target, a is forgotten, another b contact is added and b is replaced by d, and the combatant is reset
- **THEN** the contacts are c, b, then c, d, b, the target and attention target are cleared, the other target becomes d and survives the reset, and the contacts are empty

### Requirement: Queued step clears
The combat clear SHALL drop the queued step's command, lock and stop (not its pause), mark the body not moving, clear movement flag bit 2 and set bit 5, and put a move point farther than `8·speed + 20` cm at that distance straight ahead of the body. The plain clear SHALL also drop the pause and SHALL only clear bit 2.

#### Scenario: Speed 2
- **WHEN** a paused, locked, stopped, moving step with flags 6 and a move point 100 cm to the side is cleared for combat at speed 2, and then cleared plainly with flags 6
- **THEN** the pause stays, the flags are 0x22 and the move point is 36 cm ahead, then the pause is dropped and the flags are 2

### Requirement: Nav footprint
The footprint cell SHALL be `size/2 + (pos/30 − 0.5)` per horizontal axis, rounded to the nearest with ties to even. A cell closer than five to the edge on x SHALL be replaced by 8 on x without checking y, else a bad y by 8 on y, and the footprint SHALL NOT be stamped; the body SHALL be flagged off the grid once, and the overflow SHALL be logged once per run.

#### Scenario: Ties and edges on a 100-cell grid
- **WHEN** cells are stamped at 0 cm, at (30, 60) cm, far along +x, far along −z, at 1365 cm and at 1335 cm on x
- **THEN** the cells are (50, 50), (50, 52), (8, 50) off the grid, (50, 8) off the grid, off the grid and on it, and only the first off-grid stamp flags the body

### Requirement: A blow on a combatant
A blow SHALL do nothing to a character that is down, dead or being removed. A clean hit SHALL lower its courage to `((t1 + 1)·0.025 + 0.95)·courage`. An attacker without the mark SHALL take the struck character's combat rating as its struck rating when higher, and SHALL be marked.

#### Scenario: Two hits
- **WHEN** a character with combat rating 30 takes a clean hit from an attacker whose struck rating is 10, then a glancing hit at rating 50, then a clean hit while down
- **THEN** its courage is lowered once, and the attacker's struck rating is 30 and it is marked

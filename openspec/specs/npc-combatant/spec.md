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

### Requirement: The fighting estimate's precision
The fighting estimate SHALL build `levels·0.4 + 2` in double precision and cut it to a float, add `combat_skill·0.02` in double precision, add each weapon byte times the double 0.01 to the float sum in double precision, add `0.02·sum·reach` in double precision, then add 2 when the body scale exceeds the double 1.3 and otherwise quarter it.

#### Scenario: Nine levels
- **WHEN** a record has nine skill levels and combat skill 0
- **THEN** the base is bits 0x40b33333, not the 0x40b33334 a float sum gives

### Requirement: The measured origin of a combat place
The position score SHALL measure a place from the scored combatant's origin: down or dead its body; moving, its chest plus its heading times 70 ticks of its speed (times one less its phase while attacking); stepping, its body moved toward its step goal by half its own skill and then 10 cm ahead; otherwise 10 cm ahead of its body. A thrust whose arc holds the observer SHALL scale the approach by `(2·skill + 1)·2` with the scored combatant's own skill.

#### Scenario: Leaning and stepping
- **WHEN** a body at the origin with its chest at (0, 0, 100) and heading +x moves at 2 cm a tick, attacks at phase 0.25, or steps toward (100, 0, 0) with skill 0.5 and heading +z
- **THEN** the origin is (140, 0, 100), (105, 0, 100), and (25, 0, 10)

### Requirement: The published arc's look and the head wrap
The arc publisher SHALL build its edges from the head look's yaw and publish the body's facing yaw, 20° more with a held weapon, as its look. The head tracker's wrap SHALL compare the widened float with the double π and add or take the float full turn, and the flat look SHALL be the look scaled by the reciprocal of its flat length.

#### Scenario: The float pi wraps
- **WHEN** the head tracker wraps the float π
- **THEN** it becomes π minus the float full turn, while the float just below π is kept

### Requirement: The hit reaction's concern rise
A blow from an attacker the combatant has a contact for SHALL raise that contact's concern, when it is under 15, by the threat estimate plus 0.5, plus 0.5 more for a kind-1 hit or 0.25 for a kind 2, floored at zero and scaled by one less the contact's attitude times 0.01 in double precision, capped at 15.

#### Scenario: A kind-1 hit from a half-friendly attacker
- **WHEN** a contact with attitude 50 and concern 1 is hit with kind 1 and threat 0
- **THEN** its concern becomes 1.5 and its concerned event is raised

#### Scenario: A NaN concern
- **WHEN** the contact's concern is NaN
- **THEN** the hit adds nothing

### Requirement: The hit reaction's threat estimate
The threat estimate SHALL be one less the largest share `dot / d²` among the observer's nearby living bodies other than the attacker, where `d` is the body's aim point less the attacker's position, `d²` lies between 1 cm² and the square of the attacker's radius plus 100 cm, and the dot with the attacker's look is not negative.

#### Scenario: One body ahead of the attacker
- **WHEN** the only nearby body's aim point lies 100 cm along the attacker's look and the attacker's radius is 40 cm
- **THEN** the threat estimate is 0.99

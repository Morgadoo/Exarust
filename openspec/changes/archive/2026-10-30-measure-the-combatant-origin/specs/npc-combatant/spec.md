## ADDED Requirements

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

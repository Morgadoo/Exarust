## ADDED Requirements

### Requirement: Grip runtime
Each tick a held item's grip mode SHALL follow its holder's clip (a downed body 0, Idle or Walk 2, Stab 3, other clips 1), running the grip callback when it changes, and the post-change relax pass SHALL run while the links relax.

#### Scenario: Walking two-hander
- **WHEN** the holder of a two-hander plays Walk
- **THEN** the grip takes mode 2 and the arm link runs

### Requirement: Flip step
A flip SHALL start only on a request for the holder and step only while active, by (1 − (0.5 − f)²·4)·0.05 + 0.002 (the ease single, the scale and offset double). It SHALL hold while the holder attacks more than 0.49 from the middle, and land past 0.99 or below 0.01, all double compares, a NaN running on. The record SHALL then be rebuilt from the landed side.

#### Scenario: Attack near the end
- **WHEN** an attacking holder's flip stands at progress 0.99
- **THEN** the flip holds, since 0.99 in single precision is past 0.49 from the middle in double

#### Scenario: Flip lands
- **WHEN** a flip request runs to its end
- **THEN** the item is flipped, the flip is inactive and the record is rebuilt from the flipped side

### Requirement: Parry weld
A held weapon SHALL get a stiff disabled world weld at its origin, and a soft disabled weld of strength 0.05 only when its holder may parry. The soft weld point SHALL be 0.3 times the top of the item's node box, in double, or the grip itself for a polearm. The death stiffening SHALL set a soft weld at a positive index to strength 1.

#### Scenario: Holder in the heavy-claws state
- **WHEN** a holder in the heavy-claws state holds a weapon
- **THEN** the weapon gets the stiff origin weld alone

### Requirement: Base collision sound
A non-striking held item's contact SHALL ring the base collision sound when the strength S is over 0.5 and either its clock has passed or S beats its last threshold. The volume SHALL be 1.5·v with v = min(0.01·S, 1), and the pitch (0.02·m + 0.9)·(0.6·v + 0.7) for mobility m.

#### Scenario: Full strength
- **WHEN** a strength of 100 rings on a body of mobility 0.5
- **THEN** the volume is 1.5 and the pitch 1.183

### Requirement: Base collision sound clock
Under mobility 0.1 the base sound's volume SHALL take the factor 8·m + 0.2 and its pitch 7·m + 0.3 in place of 0.02·m + 0.9. The threshold SHALL become 2·S and the clock now + 220 − round-half-even(170·v) plus a spread below 100 ms, drawn only when the sound rings.

#### Scenario: Half rounds to even
- **WHEN** a strength of 5 rings
- **THEN** the cooldown before the spread is 212 ms

### Requirement: Camera kick
A hard overhead by an attacker of scale over 1.3, past 0.6 of its clip with an impact over 60 and not yet struck, SHALL kick the chase camera upward by (2000 − d)·0.001 cm when its distance d is under 1800 cm.

#### Scenario: Far away
- **WHEN** such an overhead lands 19 m from the camera
- **THEN** the camera is not kicked

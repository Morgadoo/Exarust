## ADDED Requirements

### Requirement: Counter hold from an attack
On an attack tick of a fighter with a weapon and a shield whose combat target plays the motion its last parry was picked for, the armed parry SHALL keep aiming the held weapon's parry link at the target's blade, unless the fighter lacks Ward and its clip is at 0.4 or past (compared in double, a NaN progress too), when the parry's hold SHALL be cleared: the countered motion, the hands' arm bit and the parry links. A fighter with two weapons SHALL keep aiming on the same test.

#### Scenario: Without Ward
- **WHEN** a shield fighter's counter attack is at 30 %, then at 40 %, of its clip
- **THEN** its parry link aims at the target's blade, then its hold is cleared

#### Scenario: With Ward
- **WHEN** a shield fighter with Ward counters at 50 % of its clip
- **THEN** its parry link still aims and the countered motion is kept

### Requirement: Two-weapon hand guard
On every attack tick of a fighter with two weapons the hand that does not strike and its parent part SHALL take the arm bit: the right hand for the motions 8 through 10, the left for the others; leaving the attack's clip SHALL clear both hands' bit whether or not a parry was held.

#### Scenario: A main-side swing
- **WHEN** a two-weapon fighter swings with motion 5, then returns to its stance
- **THEN** the left hand and its forearm carry the arm bit, then neither does

### Requirement: Swing rate at a NaN progress
The swing's rate SHALL take the first-key shape (1 − 0.833·modulation) and the feral ramp's smoothstep term for a NaN progress, as the original's unordered compares do.

#### Scenario: A NaN clock
- **WHEN** a plain swing's rate is asked at a NaN progress
- **THEN** it is its rate at the first key

### Requirement: Placement clinch clear
A placement on the ground SHALL first end the body's parry and its countered motion, clear both hands' arm bit and disable every held item's parry link, without starting the stance again.

#### Scenario: A placed parrying body
- **WHEN** a body that parries is placed on the ground
- **THEN** it no longer parries and its hands carry no arm bit

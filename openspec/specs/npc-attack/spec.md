# npc-attack Specification

## Purpose
The blow an NPC picks once a fight has started: overhead, thrust, or a side.

## Requirements

### Requirement: Fight blow
An unarmed fighter inside eight tenths of its reach SHALL be able to swing overhead, and outside that reach SHALL be able to take the other side. An armed fighter stepping forward slowly SHALL thrust when its preference clears the draw, and SHALL take the overhead when its age is at least a half. A facing score above 4 SHALL take the other side. A quadruped beyond eight tenths of reach SHALL be able to thrust. Energy under eight tenths of the maximum SHALL refuse the swing.

#### Scenario: Reach, preference and energy
- **WHEN** an unarmed fighter at full energy is inside eight tenths of reach and the generator starts at 0, then the same fighter is outside that reach, then an armed fighter stepping forward with preference 9, then that fighter at age one half, then a fighter whose attack phase is 100, then a quadruped beyond eight tenths of reach, and the outside unarmed fighter is tried again at 0.79 of its energy
- **THEN** the blows are overhead, the other side, thrust, overhead, the other side and thrust, and the last try starts nothing

### Requirement: A fighter stands on the strike plateau
A fight score SHALL be 0.25 at 25 cm when the reach is 50 cm and the size is 1, and 1 from 120 cm through 130 cm when the step is 80 cm. At 600 cm the strike term SHALL be 0 and the fighter SHALL keep closing. The close term SHALL be 0 one centimetre past reach plus 150 cm. Outside a fight the close term SHALL double and the strike term SHALL be 0. A large opponent SHALL make 100 cm score 1. Wanting melee SHALL double 0.25 to 0.5. The duellist SHALL stand at 1.20 m and walk at 2 m.

#### Scenario: Plateau at 120 cm, walk at 2 m
- **WHEN** a fight with reach 50 cm, size 1 and a step of 80 cm is scored at 25 cm, 120 cm, 130 cm and 600 cm, the same spot is scored outside a fight, a large opponent is scored at 100 cm, melee doubles 25 cm, and a duellist is placed at 1.20 m and at 2 m
- **THEN** the strike terms are 0.25, 1, 1 and 0, only 600 cm keeps closing, the close term past the limit is 0, the outside-fight strike term is 0 and its close term is doubled, 100 cm scores 1, melee scores 0.5, the duellist stands at 1.20 m, and it walks at 2 m

### Requirement: A side swing scores danger
A side swing on its arc end at phase 0.5 SHALL score danger bits 0x40000000 at skill 0 and 0x40800000 at skill 1. Phase 0.35 SHALL score bits 0x3ff28f5c. Phase 0 and a sweep that is off SHALL score 0. A held weapon SHALL keep bits 0x40000000 past the limit. Outside a fight the strike SHALL be 0 and that danger SHALL stay.

#### Scenario: Edge at phase 0.5 and 0.35
- **WHEN** a side swing on its arc end is scored at phase 0.5 with skill 0 and skill 1, at phase 0.35, at phase 0, not sweeping, past the limit with a held weapon, and outside a fight
- **THEN** the danger bits are 0x40000000, 0x40800000 and 0x3ff28f5c, phase 0 and the sweep that is off score 0, the held weapon keeps bits 0x40000000, and outside a fight the strike is 0 and the danger bits stay 0x40000000

### Requirement: A thrust and a feral fighter score the place
A thrust at 100 cm SHALL score danger bits 0x3f000000 inside its arc and 0x40800000 outside it. At 200 cm danger SHALL be 0. Feral 1 SHALL halve the close term and score no danger. Dull SHALL keep the close term and score no danger.

#### Scenario: Thrust arc, feral and dull
- **WHEN** a thrust is scored at 100 cm inside its arc, at 100 cm outside it, and at 200 cm, and a side swing is scored at feral 1 and dull
- **THEN** the thrust danger bits are 0x3f000000 and 0x40800000, 200 cm scores 0, feral 1 halves the close term and scores no danger, and dull keeps the close term and scores no danger

### Requirement: A fighter scores the approach
At 25 cm and at 200 cm, with reach 50 cm, size 1 and skill 0, the approach SHALL be bits 0x3ec00000. A wide fighter at 25 cm SHALL score bits 0x3f88b000. At 400 cm, and outside a fight, the approach SHALL be 0.

#### Scenario: Near, wide and past the band
- **WHEN** a fight with reach 50 cm, size 1 and skill 0 is scored at 25 cm, 200 cm and 400 cm, the same spot is wide at 25 cm, and 25 cm is scored outside a fight
- **THEN** the approach bits are 0x3ec00000 at 25 cm and at 200 cm, the wide score is bits 0x3f88b000, and 400 cm and the outside fight are 0

### Requirement: A recovery pulls the close term
A recovering swing at 25 cm, reach 50 cm and size 1, SHALL score close bits 0x403c0000 and approach bits 0x3fc00000.

#### Scenario: Recovering at 25 cm
- **WHEN** a recovering swing is scored at 25 cm with reach 50 cm and size 1
- **THEN** the close bits are 0x403c0000 and the approach bits are 0x3fc00000

### Requirement: A weapon hand publishes the arc
Action 5 at phase 0 SHALL publish kind 1, start bits 0x3fdf66f3, end bits 0xc01c61aa and look bits 0xbeb2b8c4. At phase 0.5 a hand on the side SHALL set the end to bits 0x3fc90fdb and keep that start. A recovering swing of that action SHALL publish kind 4 and keep that end.

#### Scenario: Early edge and a hand at phase 0.5
- **WHEN** action 5 is published at phase 0, again at phase 0.5 with a hand on the side, and again while recovering
- **THEN** phase 0 is kind 1 with start bits 0x3fdf66f3, end bits 0xc01c61aa and look bits 0xbeb2b8c4, phase 0.5 keeps that start and sets the end to bits 0x3fc90fdb, and the recovering publish is kind 4 with that end

### Requirement: A facing scales the place
A fighter who is not attacking, with the candidate beside it, SHALL score approach bits 0x3f400000 at 25 cm. Straight ahead SHALL keep bits 0x3ec00000. A press with the opponent facing aside SHALL score strike bits 0x40300000 and approach bits 0x40840000.

#### Scenario: Beside, ahead and a press
- **WHEN** a fight at 25 cm is scored while not attacking with the candidate beside it, then straight ahead, then pressing while the opponent faces aside
- **THEN** the approach bits are 0x3f400000 and 0x3ec00000, and the press scores strike bits 0x40300000 and approach bits 0x40840000

### Requirement: An idle stamp recovers the arc
An armed action outside a swing SHALL publish kind 4 when the idle stamp is 5 and the clock is 4, with both edges on the heading. Stamp 4 against clock 4, and stamp 0 against the maximum clock, SHALL publish kind 0.

#### Scenario: Stamp above the clock, and not above it
- **WHEN** an armed action outside a swing is published with stamp 5 and clock 4, with stamp 4 and clock 4, and with stamp 0 and the maximum clock
- **THEN** the first publish is kind 4 with both edges on the heading, and the other two are kind 0

### Requirement: The approach peaks at the strike ideal
Inside the strike ideal the approach fraction SHALL be the distance over the ideal, so the term is 1.5 at the ideal; its floor SHALL compare the fraction with 0.1 in double precision, and the eased value SHALL be `(3 − 2f)·f²`. A recovering swing SHALL lower the close term outside a fight too.

#### Scenario: At, inside and at the floor of the ideal
- **WHEN** a fight with reach 50 cm and size 1 is scored at 50 cm, 10 cm, 5 cm and 4 cm, and a recovering swing is scored at 25 cm outside a fight
- **THEN** the approach is 1.5, about 0.016224, about 0.001176 (the f32 fraction 0.1 lies above the double 0.1) and 0, and the recovering close term is one less, doubled

### Requirement: The observer's heading and a thrust's arc scale the approach
When the observer is not attacking the approach SHALL be multiplied by one plus one minus the cosine of the candidate off the observer's heading; the opponent's facing SHALL NOT enter that scale. A live thrust whose arc holds the observer SHALL multiply the approach by `(2·skill + 1)·2` with the opponent's skill, past the danger gate too, unless the candidate inside the gate lies outside the arc. The observer's feral level SHALL scale the danger and the close term.

#### Scenario: A thrust past the gate
- **WHEN** a live thrust whose arc holds the observer is scored at 200 cm (past its 110 cm gate) with the candidate outside the arc, and again with the observer outside the arc
- **THEN** the first scores no danger and approach bits 0x3f400000, and the second approach bits 0x3ec00000

### Requirement: The score's double-precision steps
The live test SHALL compare the phase with `0.3 − 0.15·skill` in double precision; angle wraps and the arc midpoint's bound SHALL test ±π in double precision; the side swing's wait at phase 0.4 or less SHALL be summed in double precision, the thrust's wait SHALL be `(0.6 − phase)·1.5` in double precision, and the danger ramp SHALL be `4 − ms/300` in double precision for 900 ms or less. The recorded bearing SHALL be that of the opponent's point seen from the candidate.

#### Scenario: Bounds at the f32 values
- **WHEN** the f32 pi is wrapped, a midpoint starts at the f32 pi, a phase of 0.3 is tested at skill 0, and candidates behind, in front of and to the −x side of the point record their bearing
- **THEN** the f32 pi wraps by a turn, the midpoint spans across the cut, the phase is live, and the bearings are about 0, −π and about π/2

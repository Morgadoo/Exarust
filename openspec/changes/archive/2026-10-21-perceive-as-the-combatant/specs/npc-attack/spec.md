## ADDED Requirements

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

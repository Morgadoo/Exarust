## MODIFIED Requirements

### Requirement: Gait plant phase
A walking foot's phase SHALL be the fraction of clip progress + 1.2 − 0.5×leg, summed in double. A free foot SHALL plant unless the phase is over the double 0.4, and under 0.2 only when it moves along the facing at most 0.3 cm a tick (backpedalling, back at most 0.2); a NaN plants. A touching foot SHALL lift unless the phase is strictly between 0 and 0.55. Any other foot SHALL keep its state.

#### Scenario: Progress zero
- **WHEN** the walk's progress is 0
- **THEN** leg 0 is at phase 0.2 and leg 1 at phase 0.7; a free leg 0 plants and a touching leg 1 lifts

#### Scenario: A slow foot plants early
- **WHEN** a free walking foot at phase 0.1 moves forward 0.29 cm a tick, and another 0.31
- **THEN** the first plants and the second keeps its state

### Requirement: Stance score
The stance score SHALL be 2(1 − C²) + ((1 − W²) + A). A SHALL be 1 minus the reference's horizontal distance to the foot segment times the single 1/30, less a double 0.1 a cm of a crouched span past 120 − 30·crouch. W SHALL be |60·scale − span| / (60·scale). C SHALL be the same for 55·scale and the span's dot with the character frame's side axis, rooted when positive.

#### Scenario: Natural and crossed
- **WHEN** the feet are 60 cm apart, the reference lies on the segment, and the frame's side axis runs from the first foot to the second
- **THEN** the natural order scores 2.524 and the swapped order scores −4.744

## ADDED Requirements

### Requirement: Walking ground weights
Each tick a body SHALL smooth a stored foot height difference 0.9/0.1 in double toward g0 − g1 (g1 − g0 past half the phase frac(progress + 0.45)), from last tick's grounds, while it moves under its own power; else it SHALL keep 0.9 of it. Walking, its ground SHALL be the feet's grounds weighted (1 − r)/2 + (1 − |2t − 1|)·r and the rest, r = 1 − run/2, t = frac(progress + c), c 0.4 over 0, 0.65 under −20 cm, else 0.4 − 0.0125·difference.

#### Scenario: A walking stride
- **WHEN** a body walks at progress 0 with grounds 10 and 20 cm and no stored difference
- **THEN** the difference becomes −1 cm and the first foot weighs 0.825

### Requirement: Imbalance accumulator
While balance is not under the double 0.2, a foot gives support and no shock stagger is on, a chest past the step trigger SHALL add (h − R·E⁴S)/(5·scale) × the double 0.005 in double, capped at 1; each tick the feet routine SHALL take the double 0.01 off, floored at 0. Nothing reads it.

#### Scenario: Past the trigger
- **WHEN** the chest is 50 cm out, R·E⁴S is 30 cm, the scale 1 and the accumulator 0.5
- **THEN** it becomes 0.52, and 0.51 after the feet routine

### Requirement: Footwork reach
The footwork's stance origin, foot scores and anchor clamp SHALL use the character's step reach, 80·√scale·(1 − 0.2·encumbrance)·(1 − 0.3·feral) cm in double, the encumbrance factor applied first, not a length scaled by the leg extension.

#### Scenario: Armoured
- **WHEN** a human of scale 1, not feral, wears encumbrance 0.5
- **THEN** its footwork reach is 72 cm

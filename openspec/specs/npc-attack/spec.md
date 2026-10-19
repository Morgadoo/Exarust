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

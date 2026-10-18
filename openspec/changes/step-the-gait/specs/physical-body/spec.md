# Spec Delta

## MODIFIED Requirements

### Requirement: Foot contacts
Each leg SHALL keep a foot contact (bits 2, 4, 8, 0x10, 0x20, 0x100) and an anchor. Not walking, a foot that does not touch is planted where it is once it meets the level. Walking, phase is the fraction of progress + 1.2 − 0.5×leg: a free foot plants on its ankle across the ground when the phase is 0.2 to 0.4, a touching foot lifts when the phase is 0 or at least 0.55, and any other foot keeps its state. Standing still: lying, none touches; getting up (to 0.9) or unsupported, each touches on its stance spot; else one foot at a time steps to a spot 5 cm off.

#### Scenario: Standing
- **WHEN** a body stands on a floor for 120 ticks
- **THEN** both feet touch (state 0x2), anchored where they landed at the floor plus the sole height, their grips full

#### Scenario: Walking and stopping
- **WHEN** a standing body walks at progress 0, then stops
- **THEN** the near foot stays touching and the far foot lifts; stopped, the lifted foot is planted where it is and the touching foot keeps its anchor

#### Scenario: Getting up
- **WHEN** a body gets up
- **THEN** its start clears every foot's state; in the first fifth of the clip no foot touches, then each touches on its stance spot until nine tenths

#### Scenario: Lying
- **WHEN** a body lies down
- **THEN** no foot touches and its lower legs and feet have no friction

#### Scenario: A step
- **WHEN** a standing foot's stance spot is 4.9 cm, then 5 cm from its anchor
- **THEN** it stays, then steps (anchored, its anchor the spot); the other foot does not step until it has landed, and a stalled step lands where it is

### Requirement: Foot friction
Each tick a living body's lower legs and their feet SHALL take their friction from their foot's state: 0 while it does not touch, 2.0 touching, 0 anchored.

#### Scenario: By state
- **WHEN** a standing body's feet touch, then it walks at progress 0
- **THEN** its lower legs and feet have friction 2.0, then 2.0 on the near foot and 0 on the far foot

## ADDED Requirements

### Requirement: Knockback step target
A knockback step SHALL keep the target the blow set, including while the body walks, until balance is under 0.2 or it is getting up.

#### Scenario: Walking through it
- **WHEN** a knockback target is 20 cm ahead and the body is walking with balance 1
- **THEN** the step stays on that target

#### Scenario: Balance gone
- **WHEN** that step's balance falls under 0.2
- **THEN** the step ends

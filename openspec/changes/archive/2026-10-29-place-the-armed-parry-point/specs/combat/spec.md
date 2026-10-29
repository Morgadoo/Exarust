## ADDED Requirements

### Requirement: Armed parry blade height
The armed parry SHALL meet the attacker's blade at the blade length less 20 cm when the chest gap less 20 cm is beyond the attacker's reach, and otherwise at the blade length less the unused reach less 20 cm, floored at 0; a NaN gap SHALL take the second branch.

#### Scenario: Within reach
- **WHEN** the chests are 100 cm apart, the reach is 100 cm and the blade 90 cm
- **THEN** the parry meets the blade 50 cm up

### Requirement: Armed parry sweep
The armed parry's sweep direction SHALL be the blade point's travel this tick, its height scaled by 16 − 16·clock outside the stab and the overhead, less its part toward the attacker's chest, normalised with the approximate reciprocal square root.

#### Scenario: Side swing
- **WHEN** a side swing at clock 0.75 rises 1 cm and travels 1 cm across
- **THEN** the sweep runs 4 up for every 1 across

### Requirement: Armed parry point
The parry point SHALL be the blade point moved along the sweep by the defender chest's lead less 60 cm (80 cm for an overhead), floored at 0; when the blade point already lies further along the sweep than the defender's chest the blow SHALL be passed and the point SHALL be 10 cm on. The falloff SHALL read the point's ground distance from the defender's chest.

#### Scenario: Blade ahead of the chest
- **WHEN** a stab's blade point sweeps toward a defender chest 100 cm along the sweep
- **THEN** the point sits 40 cm along the sweep and the blow is not passed

#### Scenario: Blade past the chest
- **WHEN** the defender's chest lies 5 cm behind the blade point along the sweep
- **THEN** the blow is passed and the point is 10 cm on

### Requirement: Armed grip arithmetic
A weapon's grip strength SHALL be (1.4 − handling) × (falloff × 0.75) in double, stored as a float, then × 0.6 in double; a shield's SHALL be stamina × (weight × (falloff × 0.75 × 0.4)) in double. A NaN distance SHALL give a NaN falloff, and the parry's blend SHALL be 20 × its speed rounded half to even.

#### Scenario: Half-way blend
- **WHEN** the parry speed is 0.625
- **THEN** the blend is 12 frames

### Requirement: Stance by energy
The stance a fighter returns to by its energy SHALL be the Defence stance when its yellow health is under 0.01 or its most energy less 0.05 exceeds its energy, both compared in double, and the combat idle otherwise; a NaN energy SHALL give the combat idle.

#### Scenario: Energy at the threshold
- **WHEN** a fighter's energy is the float 0.95 and its most is 1
- **THEN** it takes the Defence stance

## ADDED Requirements

### Requirement: Step phase
A foot's step phase SHALL advance by 0.0002 times the square root of the squared travel plus the phase, computed in double, only when the squared travel exceeds the engine's epsilon (bits 0x33ffffd7), and wrap by subtracting 1 above 1.

#### Scenario: Wrap
- **WHEN** the phase is 0.99995 and the travel is large enough to add 0.0002
- **THEN** the phase wraps to a small positive value and the previous phase is reduced by 1

### Requirement: Point push
A push at a world point SHALL be shared among the body part's particles by the point's weights, scaled by 1 minus four times the dot of the push with itself over the squared per-tick travel of the point (z+(y+x) order), one when the scaled value is under epsilon and zero when the travel is zero.

#### Scenario: Still point
- **WHEN** the point has not moved this tick
- **THEN** the push is not applied

### Requirement: Femur posture
In the combat conditions the thighs SHALL be turned about the axis (0, 0, 1) lerped 0.4 toward (-1, 0, 0) by 5 degrees with the splay flag or (1 - record height) times -1.5 degrees otherwise, the sign alternating by leg; outside them the clip's turn stays.

#### Scenario: Standing
- **WHEN** the body is not in a combat condition
- **THEN** the thighs keep the clip's turn

### Requirement: Swing body rules
The swing routine's body side SHALL follow the executable's order of float operations: the weapon-in-hand gate on the squared distance, the swing sound, the fatigue ramp, the attack arms' stiffness by stance, the fist, edge, overhead and homing nudges, the two-hand group bit, Break Tangle and the clinch clear.

#### Scenario: Unarmed fist
- **WHEN** the body swings with no weapon
- **THEN** the fist nudge applies and the weapon gate stays closed

## ADDED Requirements

### Requirement: Combat aim gate
In combat mode the point under the cursor SHALL become the combat aim only when its distance from the player across the ground is 50 cm or more; a nearer point, or a NaN distance, SHALL keep the last aim.

#### Scenario: Fifty centimetres
- **WHEN** the cursor's point is 50 cm ahead of the player, then 49.9 cm, then straight above the player
- **THEN** the first sets the aim, and the other two keep it

### Requirement: Stored auto-target
Each combat tick the player's behaviour SHALL store as its target the visible living opponent within 500 cm whose score `(500 − d)·(cursor·dir/d + 0.5)` is strictly the highest above zero, and SHALL clear it outside combat mode; the aim point's bone lock SHALL read this target.

#### Scenario: Nearer opponent ahead
- **WHEN** one opponent stands 2 m ahead and another 1 m ahead
- **THEN** the stored target is the one 1 m ahead, and none once combat mode ends

### Requirement: Deep-crouched step
Past a crouch of 0.5 the footwork's step SHALL be 0.7 of the stored reach, multiplied in double precision and narrowed once, then multiplied by the legs' extension, whatever the long-step latch; a NaN crouch SHALL keep the mode's step.

#### Scenario: Extension after the share
- **WHEN** the reach is 72 cm, the extension 0.9 and the crouch 0.6
- **THEN** the step is the single 45.36 with bits 1110798500

### Requirement: Movement step vector
A movement word's step vector SHALL sum the facing (W), the side vector `(facing.z, 0, −facing.x)` unscaled (A added, D subtracted) and the negated facing (S), or take the pad's direction, drop the height, and scale it by `step·factor / length`; the movement goal's direction SHALL use 0.8 of the side vector and drop the height, and its tapped stride SHALL be `(1.5 − 0.3·encumbrance)·reach` in double precision, then the extension, then the clock factor.

#### Scenario: Diagonal step
- **WHEN** W and A are held facing +Z with an 80 cm step and factor 1
- **THEN** the step vector is 56.5685 cm on x and on z

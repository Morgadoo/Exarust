## ADDED Requirements

### Requirement: Walking feet lag
In the walking condition each leg's lead SHALL be its animated ankle less its physical contact along the facing (negated walking backwards), in cm, or 0 when not positive. Each positive lead SHALL multiply the push's lag factor, from 1 each tick, by 1 − (lead/(60·size))², or set it to 0 at or past 60·size (a NaN limit cuts). The carry's push SHALL read the factor last measured.

#### Scenario: Two late feet
- **WHEN** both walking feet trail their animated ankles by 30 cm at size 1
- **THEN** the lag factor is 0.75² = 0.5625

#### Scenario: Backwards
- **WHEN** a body walks backwards and both feet are 10 cm behind their animated ankles along the negated facing
- **THEN** the lag factor is (1 − (10/60)²)²

### Requirement: Late foot swing flag
A walking, touching foot SHALL lose 0.3 cm of swing clearance a tick in double precision, floor 0, keep no step lift, and take the swing flag when its lead times the pelvis alignment is over 10·scale·(0.25 − frac(0.5·leg + progress + 1)); under a run blend of 0.01 (in double) that threshold SHALL first add the gait's ground difference and take the processor's minimum with 25 cm. Any other grounded foot SHALL clear the flag.

#### Scenario: Thresholds at progress 0
- **WHEN** both touching feet lead by 2 cm at progress 0, scale 1, alignment 1 and no ground difference
- **THEN** leg 1 (threshold −2.5 cm) takes the swing flag and leg 0 (threshold 2.5 cm) does not

#### Scenario: Cap
- **WHEN** the threshold plus the ground difference is 42.5 cm, or NaN
- **THEN** the threshold is 25 cm

### Requirement: Ground Query
A foot's floor query SHALL be a point ray from 30 cm over its ground 60 cm down under the toe against floor-flagged pieces only, the highest upward face under the point. Its hit SHALL feed the sole floor and raise a moving foot's ground to a higher hit. Level colliders without the floor bit SHALL not be met.

#### Scenario: Unflagged level
- **WHEN** a foot stands over the game's level colliders, none floor-flagged
- **THEN** the query misses and the floor is the ground plus the leg's offset

#### Scenario: Flagged step
- **WHEN** a moving foot's toe is over a floor-flagged step top 15 cm above its ground
- **THEN** the foot's ground and floor rise to the step top

### Requirement: Placement leg snap
A living body placed into the scene SHALL probe each leg 5·scale cm along its shin from its clip ankle with a 10 cm sphere from the frame's height plus the leg's length 300 cm down, a miss 10 000 cm under the frame, and average the hits into the character's ground. Each anchor SHALL be its clip ankle at its ground plus the sole, its ground and floor that ground, and the standing leg IK SHALL write its corrections at once with no rate and no thigh turn.

#### Scenario: A platform
- **WHEN** a biped is placed over a platform 15 cm up
- **THEN** each leg's ground and floor are 15 cm, each anchor 15 cm plus the sole, the character's ground 15 cm, and the corrections' rates zero

### Requirement: Placement frame height
The placement's frame height SHALL be the root over the chosen thigh plus the chosen ground, the clip reach (from the ankle's horizontal offset from the hip, shaped r² + (1 − r)√r) and the sole. The lower leg SHALL be chosen unless its clip foot hangs too far under its hip.

#### Scenario: No ground
- **WHEN** no probe finds ground
- **THEN** the body is left as it was and no height is given

### Requirement: Body tick adapters
At the top of each body tick the support adapter SHALL be 0.9 + 0.09·S less 0.6·(1 − E) then 0.1·(1 − B), each floored at 0 by the processor's maximum; the rest adapter 1 − 0.01·S plus 0.05·(1 − B) then 0.3·(1 − E), each capped at 1 by its minimum, and 1 while a get-up is under 0.9 of its clip; sums in double, S last tick's support.

#### Scenario: Half balance
- **WHEN** a supported, rested body has balance 0.5
- **THEN** the rest adapter is 1 and the support adapter 0.94

### Requirement: Body tick look rate and counters
The look rate SHALL rise 0.0025 a tick in double while locomoting, at most 0.15, else be 0.0125. A countdown SHALL fall one a tick and the weapon switch step one toward zero.

#### Scenario: Moving
- **WHEN** a body locomotes for 200 ticks and then stands
- **THEN** its look rate reaches 0.15 and returns to 0.0125

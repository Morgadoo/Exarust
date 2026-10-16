## ADDED Requirements

### Requirement: Blood drawn by weapons
The weapon that deals a bleeding blow SHALL draw blood at the blow: half the wound's size R more, kept within 0 to 1 and never washed off, whether the wound waits or is dropped from a full queue. Each held weapon SHALL keep its own blood. A fist, and a blow that does not bleed, SHALL draw none.

#### Scenario: Chest cut
- **WHEN** the one-handed sword cuts a human's bare chest (R 0.457)
- **THEN** the sword holds 0.2285 of blood at once, seven ticks before the wound is painted

#### Scenario: Soaked
- **WHEN** the sword deals five such cuts
- **THEN** it holds 1, and further bleeding cuts leave it at 1

#### Scenario: No blood drawn
- **WHEN** the sword cuts a skeleton, or a human's hand at 4 cm a tick (R 0.045), or a fist bleeds a fighter
- **THEN** the sword's blood stays as it was and the fist has none

### Requirement: Weapon stain
A weapon holding more than 0.01 of blood SHALL be drawn again over its model's root node's first mesh (every part of an assembled weapon), multiplying the frame by the gradient `BldGr02D` read as data, at u = 0.025·x − 0.5 (repeating) and v = 0.0125·y + o (clamped), x and y in cm in the weapon's frame, o = −(L/80)(1 − √a·√(80/L)) for tip height L and blood a.

#### Scenario: Creeping from the tip
- **WHEN** the short sword (tip 60.05 cm up its frame) holds 0.25, then 1
- **THEN** its stain begins 34.7 cm, then 69.3 cm, below the tip, densest at the tip and speckled at its edge, and the blade below the edge is unchanged

#### Scenario: Faint blood
- **WHEN** a weapon holds 0.01 of blood or less
- **THEN** no stain is drawn

#### Scenario: Assembled weapon
- **WHEN** a factory-built weapon draws blood
- **THEN** every one of its parts is stained, the gradient keyed on the whole weapon's frame and tip

### Requirement: Stain pass
The stain SHALL take the gradient's texel faded to white by the object's fade (0 for every Exarust object), write it opaque without writing depth, with the depth nudged toward the camera by 0.000 005 of the depth range so it passes against its own weapon, back faces culled, casting no shadow; white SHALL leave the weapon unchanged. The gradient SHALL read with the colour textures' LOD bias.

#### Scenario: Clean below the edge
- **WHEN** a stained sword is seen from the side
- **THEN** its blade is dark red toward the tip and exactly as without blood below the stain's edge and on the grip

#### Scenario: Occluded
- **WHEN** a hand or a body stands in front of the stained blade
- **THEN** the stain is hidden behind it as the blade is

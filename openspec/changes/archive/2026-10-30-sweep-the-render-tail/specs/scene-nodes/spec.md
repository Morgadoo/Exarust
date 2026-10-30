# Spec Delta

## ADDED Requirements

### Requirement: Light update colours
A light's update SHALL clamp an intensity below the engine's epsilon to 0 (a NaN stays) and take the direct colour: the linearised colour times the intensity, or the stored colour times `√I·0.75` in the legacy workflow. The running sum SHALL become the direct colour, with its red-first Rec. 601 luma.

#### Scenario: A tiny intensity
- **WHEN** a light updates with intensity 1e-9
- **THEN** its intensity, direct colour and luma are 0

### Requirement: Light update gates
The second colour SHALL be the direct colour unless the node word has 0x20000 (then black, and the running sum is not added to); the GI colour SHALL be the direct colour times the record's multiplier unless the node word has 0x10000. The body's flicker and fade bits SHALL NOT gate them.

#### Scenario: A flickering body
- **WHEN** a light whose body flags flicker and fade derives with node word 0
- **THEN** its second colour equals its direct colour

### Requirement: Light node colour
The update SHALL give the light node the linearised colour times `√I·M·3`, or `√(I·M)·0.6` with the 0.6 in double in the legacy workflow, M being the record's multiplier.

#### Scenario: The legacy workflow
- **WHEN** a light of intensity 4 and multiplier 0.5 updates in the legacy workflow
- **THEN** its node colour is the linearised colour times √2·0.6 rounded from double

### Requirement: Light update reach
The update SHALL report the radius as the reach unless the multiplier is below the engine's epsilon (then 0; a NaN multiplier keeps the radius), store a reach that moved by more than 1 and set pending bit 2, and set pending bit 4 on every update.

#### Scenario: A zero multiplier
- **WHEN** a light with radius 10 and multiplier 0 updates
- **THEN** its reach is 0

### Requirement: Light alias falloff
The alias's plain test SHALL be the luma of the colour product against the cutoff. The gated test SHALL require the spheres to overlap and the oriented-box distance to be under the light radius, take the larger of the box distance and the squared sphere shell (when the node's centre is outside its radius) and weigh `max(1 − d·falloff, 0)⁴`; it SHALL gate by the weight times the intensity and report the colour-product luma times the unscaled weight.

#### Scenario: A brighter light
- **WHEN** the same node is weighed by a light of intensity 4 and of intensity 1
- **THEN** both report the same weight

### Requirement: Light alias spot and faces
A spot alias SHALL multiply the weight by `max((2^(t·cone) − 1)·exponent, 0)` with the engine's fast logarithm and exponential, `t` measured from the box's lowest point; the face mask SHALL clear each cube face whose plane tests put the node beyond it by more than the node's bounding radius.

#### Scenario: A node along +z
- **WHEN** a light at the origin weighs a node centred 100 cm along +z with 10 cm half extents
- **THEN** only the face pointing back along −z stays in the mask

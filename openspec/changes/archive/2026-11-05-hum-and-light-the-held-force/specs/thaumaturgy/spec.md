## ADDED Requirements

### Requirement: Kinesis shell hum
The system SHALL give the shell a kinesis pick puts on a body the `thmfrc_knsdr` hum: when the shell joins the scene a looping voice starts at the shell node's place with volume 0 and pitch 1, its start the effect stream's draw times 0.8, then the buffer's length, in double; each frame the voice SHALL follow the node's box centre and take the shell's hum volume and pitch; it SHALL be freed with the shell. No other shell SHALL hum.

#### Scenario: A kinesis lets go
- **WHEN** a kinesis ends on its own
- **THEN** its shell's hum swells to `0.2·(2 − |1.5 − 3.5·s|)` with its pitch `0.75·s` while the shell closes, and its voice is freed when the shell leaves

### Requirement: Held shell box light
The system SHALL give every held body's shell (a kinesis pick's, a throw's grab) a `gi_cuboid` light over its node's box: the extent `max(half − 20, 10)` on each axis (MAXSS), the shell's light colour as its render step wrote it with no manager scale, `bMat` the inverse of the node's frame less the box's centre in it, in the node's world box widened 10 cm, rounded, plus 80 cm. The light SHALL leave with the shell's light.

#### Scenario: A held prop turns
- **WHEN** the held body turns a quarter about the vertical
- **THEN** the light's box turns with it

### Requirement: Strikes line light
The system SHALL give every strikes aura a `gi_gradline` light along its weapon node's y axis: with the strikes centre `c`, start `s` and length `l` and `w = 2·(c − s)`, the segment from `max(c − w, bottom)` to `min(c + w, top)` of the node box carried into the world, the gradient `g = c/l`, `((1 − c) + y₀)/w/g` and `((1 − y₁) + c)/w/(1 − g)`, attenuation 10000, in the node's world box plus 60 cm.

#### Scenario: A long node box
- **WHEN** the node box reaches past `c ± w`
- **THEN** the segment runs from `c − w` to `c + w`

### Requirement: Strikes line colour and point
The strikes line light's colour SHALL be the shell's light colour times one less the node's fade, and while the aura has a ripple its point light SHALL stand at the first ripple with ten times its shown amount, else be zero.

#### Scenario: A strikes blow lands
- **WHEN** the caster's blow ripples its weapon's aura
- **THEN** the line light's point light stands at the ripple with ten times its shown amount

### Requirement: Strikes blow ripple
The system SHALL, while Force Strikes runs, ripple the aura on its caster's primary weapon at each striker's hook the caster's blows call, at the contact in the weapon's frame and half the hook's strength, sounding `thmfrc_shi` at the contact (the aura's gain 1.5 and base pitch 0.8) when its 400 ms gate is open.

#### Scenario: The primary weapon has no aura
- **WHEN** the caster's primary weapon carries no strikes aura
- **THEN** the hook ripples nothing

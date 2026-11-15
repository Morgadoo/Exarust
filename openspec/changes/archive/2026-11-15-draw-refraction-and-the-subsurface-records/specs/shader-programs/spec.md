## MODIFIED Requirements

### Requirement: Refraction background
A liquid-mode record SHALL bind the scene refraction texture's high field above a glossiness of 0.85 and its low field otherwise when the scene's refraction switch is on (nothing without the texture), and the built-in `BkgRfr02` or `BkgRfr01` by the same test when it is off; the record's glossiness SHALL be widened to double before the compare, whatever its specular bit. Any other program with local reflections SHALL bind the scene texture's low field when there is one.

#### Scenario: Stored 0.85
- **WHEN** a liquid record with glossiness 0.85 is drawn with the switch off
- **THEN** `BkgRfr02` is bound

### Requirement: Shadow tap table
The `SdwSmp` table SHALL be the original's 32 three-float taps at their exact single-precision values, the surface shaders SHALL hold all three floats of each, and the sun shadow filter and the main light loop SHALL use each tap's first two lanes.

#### Scenario: First tap
- **WHEN** tap 0 is read
- **THEN** its bits are `0x3E8B8A8F`, `0xBF0C745B`, `0x3EB5B2CD`

#### Scenario: Ninth tap
- **WHEN** tap 8 is read
- **THEN** its bits are `0x3F3695B3`, `0x3F0394AE`, `0x3CE69C29`

## ADDED Requirements

### Requirement: Refraction scene fields
The renderer SHALL keep two fields of the previous frame for refraction: the post chain's half-size bright pass (the high field) and its quarter-size blurred bright pass (the low field), copied after each frame's post pass, and SHALL bind each refracting material to the high field for a glossiness above 0.85 and to the low field otherwise.

#### Scenario: Gem
- **WHEN** the shipped `gemcutruby1` record (glossiness above 0.85) is drawn
- **THEN** it reads the high field

### Requirement: Refraction read
A refracting fragment SHALL read its field at its screen position plus the view-space normal's push times `ViewPrj / w`: inside the GI volume r·(1.5 − r.z) with xy × 0.01·s, outside it r·r.z·0.01·s, s the record's strength kept in hundredths as a 16-bit word.

#### Scenario: Facing the eye
- **WHEN** the view-space normal is (0, 0, 1)
- **THEN** the read sits at the fragment's own screen position

### Requirement: Refraction mix
The read SHALL be weighted by min(max(1 − |a + 0.9|·1.5, 0) + (1 + a)³·0.7, 1), a the cosine between the eye-to-surface direction and the normal, tinted by mix(diffuse colour, 0.2, −0.1 − 0.2·a), scaled 8 inside the GI volume and 12 outside, and added to the lit colour times the alpha by one less the alpha; the fragment SHALL be opaque.

#### Scenario: Edge on
- **WHEN** a refracting fragment is seen edge on (a = 0)
- **THEN** the read's weight is 0.7

### Requirement: Subsurface records
Every lit record whose shader word selects the subsurface program SHALL draw the skin branch: its secondary colour the scattered share of the diffuse, the rest plain diffuse (squared `MatDif` without a diffuse or specular map), alpha the material opacity without a fall-off, no emissive colour, and the skin's blood for dirt mode 4.

#### Scenario: Wax candle
- **WHEN** the shipped `waxblue01` record is built
- **THEN** it draws the skin branch with its secondary colour

### Requirement: Fast loop shadows
In the fast light loop (the translucent program and `Shd.FastLgt`) each cube lookup SHALL be the tap's three floats times the filter radius minus the light direction, unturned and without a normal offset, against the distance lowered by a²·0.008 with a = 1 − max(N·L, 0) on the vertex normal; the sun's taps SHALL be unturned.

#### Scenario: Light facing the leaf
- **WHEN** the light lies along the vertex normal
- **THEN** the compared distance is not lowered

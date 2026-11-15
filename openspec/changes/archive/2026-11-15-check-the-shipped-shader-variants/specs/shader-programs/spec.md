## MODIFIED Requirements

### Requirement: Shadow tap table
The `SdwSmp` table SHALL be the original's 32 three-float taps at their exact single-precision values, the surface shaders SHALL hold the same first two lanes of each, and the sun shadow filter SHALL use each tap's first two lanes.

#### Scenario: First tap
- **WHEN** tap 0 is read
- **THEN** its bits are `0x3E8B8A8F`, `0xBF0C745B`, `0x3EB5B2CD`

#### Scenario: Ninth tap
- **WHEN** tap 8 is read
- **THEN** its bits are `0x3F3695B3`, `0x3F0394AE`, `0x3CE69C29`

## ADDED Requirements

### Requirement: Shadow taps follow the settings
The surface shaders SHALL take `SdwSmps` shadow and GI taps per sub-frame, `shadow_samples(shadow quality, 3)` when the scene is supersampled and `shadow_samples(shadow quality, 0)` otherwise, carried in the top byte of the light buffer's first element. A supersampled sub-frame SHALL read the light taps from `SdwSmps · PassIdx`.

#### Scenario: Shipped settings
- **WHEN** the shadow quality is 4 and the scene is supersampled
- **THEN** each sub-frame takes 2 taps, and 8 without super-sampling

#### Scenario: Higher quality
- **WHEN** the shadow quality becomes 5 without super-sampling
- **THEN** the buffer carries 16 taps

### Requirement: Single plain shadow lookup
When one tap is taken and the scene is not supersampled (the original's `{?SdwSmps?}` false), every shadow lookup SHALL be one sample along the plain direction, without the disc or the normal offset, and the GI SHALL be read once along the normal with full openness.

#### Scenario: Lowest shadow quality
- **WHEN** the shadow quality is 1 without super-sampling
- **THEN** a point light's cube is read once along the light direction

### Requirement: GI sub-frame tap window
A supersampled sub-frame SHALL read its GI taps from `PassIdx` onward, as the original's ambient include indexes them `PassIdx + j`, while its light loops read from `SdwSmps · PassIdx`.

#### Scenario: Second sub-frame
- **WHEN** two taps a sub-frame are taken and sub-frame 1 draws
- **THEN** the GI reads taps 1 and 2 and the lights taps 2 and 3

### Requirement: Translucent program
A lit record whose shader word selects class 7 (`shadetrl_f`) SHALL draw with the translucent branch: per light the diffuse is colour · N·L² with N·L signed, the highlight the fast loop's weighted by max(N·L, 0); on a two-sided back face the whole normal-map normal is negated; after the diffuse colour the red channel becomes 0.8·r + 1.5·r².

#### Scenario: Leaf lit from behind
- **WHEN** a light is behind a translucent leaf at N·L = −0.5
- **THEN** its diffuse is a quarter of the light's colour

### Requirement: Fast light loop in the PBR program
A lit PBR record whose shader word has `Shd.FastLgt` SHALL take the fast light loop: a plain N·L diffuse with no Fresnel share and the fast highlight with Fresnel 0.04 + 0.96·2^(−8·V·H).

#### Scenario: Fern
- **WHEN** the shipped `lady_fern_a01` record (extra flag `0x200`) is built
- **THEN** its material carries the fast-light flag

### Requirement: Shared colour slot alpha
A lit surface with both the diffuse and the opacity map SHALL take its alpha as MatDif.a times the diffuse map's alpha; the opacity map's alpha squared SHALL apply only without a diffuse map.

#### Scenario: Leaf cut-out
- **WHEN** a leaf's colour slot texel has alpha 0.6 and MatDif.a is 1
- **THEN** its alpha is 0.6, not 0.36

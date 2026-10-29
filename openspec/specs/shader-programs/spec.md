# shader-programs Specification

## Purpose
Decodes the original's shader libraries and generates, from the user's own
installation, the GLSL program the original compiles for a material, with
the program choice and flags its material factory and shader builder make.

## Requirements

### Requirement: Shader library decoding
The decoder SHALL read a `.flb` library's table of contents and decode each entry with the rotation cipher restarted per entry, drop up to four trailing NUL bytes when the last byte is a NUL, and keep embedded NULs. Malformed input SHALL return an error, never panic. It SHALL hold for both libraries of the installed `Resource.rpk`.

#### Scenario: Installed library
- **WHEN** the installed `shading.flb` is decoded and dumped
- **THEN** its 54 entries match an independent decode byte for byte

#### Scenario: Padding
- **WHEN** an entry ends in five NUL bytes
- **THEN** its text keeps one of them

### Requirement: Flag conditions
A condition SHALL test material and object flags as a signed field of at least 1 without any comparison, and shader and environment flags the same way unless an `=` follows before the marker's end; then it SHALL compare the field with the digit two characters after the `=`.

#### Scenario: No material comparison
- **WHEN** `Obj.VrtCol = 1` is tested on a mesh with vertex colours
- **THEN** it is false

#### Scenario: Shader comparison
- **WHEN** `Shd.DirtMode = 4` is tested with dirt mode 4
- **THEN** it holds

### Requirement: Variable and light conditions
A bare `Lights` or `ShadeQual` SHALL be false, a bare `SdwSmps` true with more than one tap or super-sampling on, a bare `DspSdws` true above zero. `Light[i].Direct` and `Light[i].Sky` SHALL be false unless `i` is below `Lights`; `Direct` SHALL hold for a directional slot and `Sky` for slot 0.

#### Scenario: One shadow tap
- **WHEN** `SdwSmps` is tested with one tap and super-sampling off
- **THEN** it is false

#### Scenario: Light past the count
- **WHEN** `Light[1].Direct` is tested with one light
- **THEN** it is false

### Requirement: Condition chains
An `&` chain SHALL take the value of its last term, reading each `&` term first without its `!`, and an else condition SHALL be a single term.

#### Scenario: Forgotten first term
- **WHEN** `Lights > 2 & Light[0].Direct` is tested with one directional light
- **THEN** it holds

### Requirement: Template expressions and printing
Expressions SHALL apply `* + - /` left to right in single precision. A result within 0.0001 of its truncation SHALL print as one, two or three characters of `'0'` plus each digit; any other result SHALL print with three decimals cut digit by digit, the last rounded to nearest with ties to even, trailing zeros trimmed but one kept after the point.

#### Scenario: Quarter and sixteenth
- **WHEN** `1/4` and `1/16` are printed
- **THEN** they read `0.25` and `0.062`

### Requirement: Program choice
The program SHALL follow the factory's shader-word class: 7 translucent, bit 2 subsurface, 3 hair, bit 0 PBR, otherwise legacy. Each material and shader bit SHALL need the record's flag passed by its type's plug-in mask and, for a map, the layer.

#### Scenario: Shipped reflection layers
- **WHEN** the four shipped material libraries are surveyed
- **THEN** all 30 records with a reflection layer keep `Mat.TexRef`

### Requirement: Alpha test and blend mode
Alpha testing SHALL need the diffuse bit, a diffuse or opacity layer, the opacity bit and the class's `0x4000`; blend mode 4 SHALL be the class's low nibble 4.

#### Scenario: Opacity map alone
- **WHEN** a record with the class's `0x4000` has only an opacity layer
- **THEN** it has `Mat.TexOpc` and no alpha test

### Requirement: Displacement shadows
Displacement shadows SHALL need a height layer, a normal map, extra flag `0x100000` and shading quality above 4. `DspSdws` SHALL be 0 without them, else 4 over the super-sampling factor and at least 1.

#### Scenario: Displacement taps at 4× super-sampling
- **WHEN** a material with displacement shadows is keyed at the 4× setting
- **THEN** `DspSdws` is 1

### Requirement: Specular GI without a reflection layer
Inside the GI volume a surface whose material state lacks `Mat.TexRef` SHALL take a quarter of the ambient as its specular GI; outside the volume every surface SHALL reflect the environment cube.

#### Scenario: Indoor stone
- **WHEN** a stone record without a reflection layer is lit inside the GI volume
- **THEN** its specular GI is a quarter of its ambient, with no cube sample

### Requirement: Program cache keys
The program cache SHALL keep one bucket per material and shader word pair and, inside it, one entry per object and environment word pair. A build that fails SHALL be stored and not retried. Interning an existing pair SHALL only raise its reference count; a new pair SHALL take the first empty bucket met while scanning, or be appended.

#### Scenario: Same material, other shader word
- **WHEN** a material word is interned under two different shader words
- **THEN** it gets two buckets, and each builds its own programs

#### Scenario: Failed build
- **WHEN** a program build returns no program
- **THEN** a second request for the same words returns no program without building again

### Requirement: Shadow tap upload window
A sub-frame SHALL upload its `SdwSmp` taps starting at tap `pass · taps`, as four times the tap count in quadrant mode or the tap count otherwise, and one tap (four in quadrant mode) when the tap count is below one.

#### Scenario: Second pass
- **WHEN** two taps are used and pass 1 uploads
- **THEN** the window starts at tap 2 and holds two taps

### Requirement: Light slot gather
A draw SHALL gather at most the smaller of six and the view's light cap from its links in list order, stopping at a link with no id or a weight below 0.004 (a NaN weight passes). Unshadowed lights SHALL take the first slots and shadowed ones the rest, each keeping last frame's slot where it can.

#### Scenario: View cap
- **WHEN** eight links are offered with a light cap of two
- **THEN** two lights are gathered

### Requirement: Light slot values
A light new to its slot SHALL write its kind's position, its colour, the depth scale `(0.97 + E)/(far · scale)` and its filter radius. A light that keeps its slot SHALL refresh only a shaped light's radius and, on a scale change, its depth scale. A reset SHALL clear the light bits and the count and keep the written uniforms.

#### Scenario: Reset
- **WHEN** the slots are reset
- **THEN** the light bits and the count clear and the written uniforms stay

### Requirement: Per-draw plan
A draw SHALL gather lights only when it has a light list and its material word has the diffuse or specular bit, and SHALL otherwise clear the six slots. It SHALL rewrite the material block's diffuse alpha to `1 − fade` only under the frame's fade bit when the two differ by more than the engine epsilon (an unordered compare does not rewrite). A half-pass draw under sun shadow mode 3 SHALL halve the last sub-frame index, truncating toward zero.

#### Scenario: Unlit material
- **WHEN** a draw with a light list has neither the diffuse nor the specular bit
- **THEN** its light slots are cleared

#### Scenario: Half passes
- **WHEN** four sub-frames are drawn for a half-pass object under sun mode 3
- **THEN** two sub-frames are drawn

### Requirement: Motion-blur shutter time
A node with the motion-blur flag SHALL get, for sub-frame `i`, the time `((i + first + 0.5)·d / factor + clock) − d·0.5` in single precision in that order.

#### Scenario: Four samples
- **WHEN** four samples cover a 0.1 s shutter at clock 2
- **THEN** the first sub-frame's time is 1.9625 and the last 2.0375

### Requirement: Fog position
`FogPos` SHALL be the eye plus half the difference to the sun camera's eye, with `w` the scene's integer height limit and `y` lowered to that limit when above it.

#### Scenario: Above the limit
- **WHEN** the midpoint's `y` is 350 and the limit is 340
- **THEN** `FogPos` is `(x, 340, z, 340)`

### Requirement: Refraction background
A liquid-mode record SHALL bind the scene refraction texture's high field above an opacity of 0.85 and its low field otherwise when the scene's refraction switch is on (nothing without the texture), and the built-in `BkgRfr02` or `BkgRfr01` by the same test when it is off; the opacity SHALL be widened to double before the compare. Any other program with local reflections SHALL bind the scene texture's low field when there is one.

#### Scenario: Stored 0.85
- **WHEN** a liquid record with opacity 0.85 is drawn with the switch off
- **THEN** `BkgRfr02` is bound

### Requirement: Light colour precision
A light's colour SHALL be scaled by `2/π` over the tap count computed in double and narrowed, then by `(1 − dimmer)·exposure`; a sun-like light's filter radius SHALL be `softness·0.5` in single precision times the double `0.005`, narrowed, whatever `FastLgt` says.

#### Scenario: Seven taps
- **WHEN** a white light of intensity 1 uses seven taps
- **THEN** its colour is 0.09094568

### Requirement: Global time lanes
The `Global` block SHALL write `ViewPos.w` and `TimeDlt.x` as the sub-frame offset times the frame scale, and `TimeDlt.y` as `−(1/factor)` times the frame scale. The sub-frame offset SHALL be 0 outside super-sampling and `0.5 − (i + 0.5)/N` for sub-frame `i` of `N`; the frame scale SHALL be 1 for a frame of at most 1/60 s and `16.666…/ms` otherwise. The exposure SHALL NOT enter these lanes.

#### Scenario: Super-sampled slow frame
- **WHEN** sub-frame 1 of 4 is drawn after a 25 ms frame with factor 8 and exposure 2
- **THEN** `TimeDlt` is `(0.125 · s, −0.125 · s)` with `s = 16.666…/25`

### Requirement: Shadow declaration callback
The builder's separate shadow-declaration callback SHALL declare one `Shadow<i>` per light slot, a 2-D shadow sampler for a slot whose kind has `0x300 == 0x200` and a cube shadow sampler otherwise, testing every slot's own kind and declaring no `vrSdw` input.

#### Scenario: Sun in slot 1
- **WHEN** three slots hold point, directional and point lights
- **THEN** `Shadow0` and `Shadow2` are cube samplers and `Shadow1` is a 2-D sampler

### Requirement: Shadow tap table
The `SdwSmp` table SHALL be the original's eight three-float taps at their exact single-precision values, and the sun shadow filter SHALL use each tap's first two lanes.

#### Scenario: First tap
- **WHEN** tap 0 is read
- **THEN** its bits are `0x3E8B8A8F`, `0xBF0C745B`, `0x3EB5B2CD`

### Requirement: Texture upload wait
The material loader's wait SHALL scan its map slots from slot 0 while each holds a texture, at most five, start every scanned texture still uploading, and poll until no scanned slot is uploading.

#### Scenario: Gap in the slots
- **WHEN** slots 0 and 1 hold textures, slot 2 is empty and slot 3 holds one
- **THEN** only slots 0 and 1 are scanned

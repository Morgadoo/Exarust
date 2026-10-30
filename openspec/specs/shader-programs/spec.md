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
A geometry with the shutter flag `0x20` SHALL, on the super-sampled path, rewrite each sub-frame projection's depth lane to `((n + 0.5)·d / N + lane) − d·0.5` in single precision in that order, with `n = 4q + i`, `N` the setting's sample count, `d` the geometry's shutter length and `lane` the first pass's lane read once before the loop, and SHALL rebuild each composite from the unchanged view.

#### Scenario: Four samples
- **WHEN** four samples cover a 0.1 shutter with lane −1
- **THEN** the first sub-frame's lane is `((0.5·0.1)/4 + (−1)) − 0.05`

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
The `Global` block SHALL write `ViewPos.w` and `TimeDlt.x` as the sub-frame offset times the frame scale, and `TimeDlt.y` as `−(1/factor)` times the frame scale, the factor being the setting's sample count. The sub-frame offset SHALL be 0 outside super-sampling and `0.5 − (4q + 0.5)/N` for quarter `q` of `N` samples; the frame scale SHALL be 1 for a frame of at most 1/60 s and `16.666…/ms` otherwise. The exposure SHALL NOT enter these lanes.

#### Scenario: Super-sampled slow frame
- **WHEN** a sub-frame offset of 0.125 is drawn after a 25 ms frame with factor 8 and exposure 2
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

### Requirement: Hair program lights
A lit record whose shader word selects class 3 SHALL draw with the hair program, where each light adds an anisotropic two-lobe specular `F·C·gh·d·π·(0.1·D(0.8, 0.4r², b·h + 0.05) + 0.3·D(0.8, 0.6r², b·h − 0.05)·MatSpc)` and a diffuse `(1 − F)·C·(0.4·N·L·MatDif + RimCol·((1.15 − |v·h|)²·max(N·L + 0.75, 0))²)`, with `F = 0.05 + 0.4·2^((−5.55473·h·v − 6.98316)·h·v)` and `RimCol = normalize(MatDif)·0.6 + 0.4`.

#### Scenario: Light behind the eye
- **WHEN** a white light, the eye and the normal all point along z with roughness 0.1
- **THEN** the diffuse is `(1 − F)·(0.4·MatDif + RimCol·0.039375²)` with `F = 0.05 + 0.4·2^(−12.53789)`

#### Scenario: Rim past the terminator
- **WHEN** a light grazes from behind the surface (N·L = 0)
- **THEN** the specular is zero and the rim still lights it

### Requirement: Hair program colour and alpha
The hair program's ambient SHALL add `amb·(1.5 − 0.75·v·n)·MatDif`, the diffuse map's colour SHALL multiply the sum only under dirt mode 4, and with a diffuse map and no specular map the alpha SHALL be `(a − 1 + MatEmi.a)/MatEmi.a·MatDif.a`.

#### Scenario: Strand alpha
- **WHEN** the diffuse map's alpha is 0.75, MatEmi.a is 0.5 and MatDif.a is 1
- **THEN** the alpha is 0.5

#### Scenario: Shipped hair record
- **WHEN** the record `harhps1` (type 3, material flags `0x720007`) is loaded
- **THEN** it takes the hair flag

### Requirement: Specular needs MatSpc
A lit record whose material word lacks `MatSpc` (`0x4`) SHALL draw no specular sum (lights and reflection) in any program.

#### Scenario: PBR record without MatSpc
- **WHEN** a lit record has material word `0x3` and shader class 1
- **THEN** it takes the no-specular flag and not the hair flag

### Requirement: Super-sampling setting table
The super-sampling setting SHALL index an eight-entry table of sample counts 1, 2, 3, 4, 8, 15, 24 and 66 with jitter offsets 0, 1, 3, 6, 10, 18, 33 and 57. A quarter SHALL draw `min(samples, 4)` sub-frames and a frame `ceil(samples/4)` quarters. Sub-frame `i` of quarter `q` SHALL take jitter entry `offset + 4q + i` with no stop at the setting's own run.

#### Scenario: Three samples
- **WHEN** the 3-sample setting is drawn
- **THEN** one quarter of three sub-frames is drawn

#### Scenario: Past the run
- **WHEN** the 15-sample setting draws its sixteenth sub-frame
- **THEN** it takes jitter entry 33

### Requirement: Sub-frame matrices
Each sub-frame SHALL build a frustum whose edges are the stored negative half extents shifted by `2·half·jitter/size` (`half + d` and `−half + d`), a view matrix that transposes the camera's axes with a translation row of `0 − t` dotted with each axis row summed `(x + y) + z`, and the composite `view · projection` summed `(a·b + a·b) + (a·b + a·b)`. A draw SHALL upload the object's matrix and the object times the composite.

#### Scenario: Rotated camera
- **WHEN** the camera's axis rows are `(0, 1, 0)`, `(−1, 0, 0)`, `(0, 0, 1)` at `(1, 2, 3)`
- **THEN** the view's translation row is `(−2, 1, −3, 1)`

### Requirement: Sub-frame animation clock
A sub-frame's clock SHALL be `i·step + base` with `step = time/samples` and `base = (q·step)·4 + (step·0.5 + time·(−0.5))` in single precision in that order.

#### Scenario: Second quarter at 24 samples
- **WHEN** quarter 1 of the 24-sample setting is drawn over a 0.1 s frame
- **THEN** its first clock's bits are `0xBCFFFFFF`

### Requirement: Per-draw emissive choice
A draw whose material key has the emissive bit and whose geometry asks for a per-draw rescale SHALL take the render object's own emissive scale when the render object's own word has `0x20000`, and the geometry record's scale otherwise, in both draw variants.

#### Scenario: Material key bit alone
- **WHEN** the material key has `0x20000` and the render object's own word does not
- **THEN** the geometry record's scale is used

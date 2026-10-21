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

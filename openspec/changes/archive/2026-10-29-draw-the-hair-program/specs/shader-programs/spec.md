## ADDED Requirements

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

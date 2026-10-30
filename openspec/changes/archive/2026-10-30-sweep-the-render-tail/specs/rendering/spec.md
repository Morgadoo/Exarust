# Spec Delta

## ADDED Requirements

### Requirement: Light view and orthographic matrices
The sun map's view matrix SHALL be column-major with rows `right = dir × up_hint`, `up = right × dir` (each normalised by the engine's normalise) and `dir`, and the translation `(−right·origin, −up·origin, −dir·origin)`; the orthographic matrix SHALL scale each axis by `2/(max − min)` and translate it by `−(max + min)/(max − min)`.

#### Scenario: A light straight down the negative z axis
- **WHEN** the view matrix is built for dir −Z, up hint +Y and origin (10, 20, 30)
- **THEN** its rows are (1, 0, 0, −10), (0, 1, 0, −20) and (0, 0, −1, 30)

### Requirement: Bloom kernel
The bloom filter SHALL round its tap count with the processor's 64-bit conversion, compute `taps + 1` weights `w = ((3n − i)/3n − 0.4375)²·24` normalised by the first weight plus twice every other, scale them by 1.5 (legacy) or `level·0.016 + 1` in double (linear), and upload the first `taps` with the last index `taps − 1`.

#### Scenario: Five taps
- **WHEN** the kernel is built for 5.4 taps
- **THEN** it uploads the first five of six weights and the last index 4

### Requirement: Post program variant
The post pass SHALL run its bright program when the sharpen amount widened to double is above 0.01 and the shader-restore switch is clear, and its plain program otherwise, a NaN amount included.

#### Scenario: A NaN sharpen amount
- **WHEN** the sharpen amount is NaN
- **THEN** the plain program runs

### Requirement: Material list lookup
The renderer's material lookup SHALL search the shadow list for a class word with bit 0x8000, the opaque list for low nibble 0, the blended list for low nibbles 1 to 4, and no list otherwise.

#### Scenario: Nibble four
- **WHEN** a class word's low nibble is 4 without bit 0x8000
- **THEN** the blended list is searched

### Requirement: Spline line widths
The line program's width values SHALL be `x = width/2 + √(4·width)` and `z = √(6·width)` in single precision and `y = −(0.3·width + 0.15)` summed in double and rounded once.

#### Scenario: Width 0.1
- **WHEN** the widths are computed for 0.1
- **THEN** y equals the double sum −(0.1·0.3 + 0.15) narrowed once

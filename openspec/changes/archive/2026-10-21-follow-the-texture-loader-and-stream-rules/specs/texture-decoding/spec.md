# Spec Delta

## MODIFIED Requirements

### Requirement: Texture sampler
The sampler SHALL follow the creation word: bit 5 linear magnification, bit 4 linear minification, bit 6 mipmapping and bit 7 linear blending between levels, bits 16 and 17 repeat (else clamp). Anisotropy SHALL be 2^(level − mode) for a mode (bits 0–1) below the quality's level and not 3, else 1, and 1 unless every filter is linear. The LOD bias SHALL be the base times (the supersampling factor times the single-precision third), evaluated in that order, where the factor is the supersampling level below 2 and that level less one from 2 up. Every material path, painted, race-skin and run-time apparel included, SHALL take the bias.

#### Scenario: A material texture at the retail settings
- **WHEN** a material with load flags 0x70000 asks for a diffuse map at texture quality 5
- **THEN** its word is 0x700F0, it repeats on every axis, filters trilinearly with 16× anisotropy and has an LOD bias of −0.75 at 4× supersampling

#### Scenario: Anisotropy modes
- **WHEN** the quality level is 4 and the word's mode is 1, 2 or 3
- **THEN** the anisotropy is 8, 4 and 1

#### Scenario: The quality table
- **WHEN** texture quality 0, 3, 4 and 5 are set
- **THEN** the mask, anisotropy level and bias base are 0x00, 0, 0; 0x70, 1, 0; 0xF0, 2, −0.5; and 0xF0, 4, −0.75

## ADDED Requirements

### Requirement: Single-channel diffuse
A diffuse map stored as BC4, the original's luminance format, SHALL be uploaded as one channel and shown as grey on every material path, with no diffuse bias for a record that has a secondary colour.

#### Scenario: A BC4 diffuse
- **WHEN** a run-time apparel or race-skin material takes a BC4 diffuse
- **THEN** the shader repeats the red channel as grey

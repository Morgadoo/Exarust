## ADDED Requirements

### Requirement: Displacement shell program
The system SHALL draw the displacement shell with the shipped `dispshell_f` (black colour, alpha from the outer rim or, by `Interior`, the darkened inside), premultiplied blending (blend word 0x163), one-sided, `EmiTex` the `_EffectShell01` record's emissive layer, each vertex moved 2 cm along its normal, and SHALL offer a depth-only form that writes depth without colour.

#### Scenario: A closed shell
- **WHEN** the shell's `Fade` is 1
- **THEN** every fragment's alpha is 0 after the output clamp

### Requirement: Scan program
The system SHALL draw the teleporter scan's aura with the shipped `effectscan_f`: fixed 0.03 coordinates, the fade term clamped to 100, no ripple points and the `Colour` uniform's ramp and alpha limit.

#### Scenario: The scan's colour
- **WHEN** a device scan is drawn
- **THEN** its `Colour` is (0.45, 0.95, 0.33, 1) times the scan's colour scale

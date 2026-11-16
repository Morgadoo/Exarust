## ADDED Requirements

### Requirement: Light colours through the original's sRGB routine
A light's direct colour and its node colour SHALL linearise the stored colour with the original's sRGB routine (the engine's fast logarithm and exponential), not the renderer's `powf` curve.

#### Scenario: The linear workflow
- **WHEN** a light updates in the linear workflow
- **THEN** its direct colour equals the original's bit for bit

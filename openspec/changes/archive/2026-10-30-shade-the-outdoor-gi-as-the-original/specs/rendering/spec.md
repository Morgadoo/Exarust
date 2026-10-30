# Spec Delta

## MODIFIED Requirements

### Requirement: Voxel GI shading
The system SHALL light surfaces inside the volume from it. Indoors the ambient is the volume's light g (20 cm along the normal, averaged over the taps) × (the level ambient × 15 × b × openness) with a = |g| and b = (1 − a)²·0.03/(a + 0.016) + (0.47a + 0.53); reflections are the environment cube × that ambient × 8 × 2.

#### Scenario: Torchlit room
- **WHEN** a level with torches has been shown for its warm-up
- **THEN** walls and floor out of a torch's direct reach show its warm bounce light

#### Scenario: Outside the block
- **WHEN** a surface lies outside the GI block, or GI is turned off
- **THEN** it takes its ambient from the environment cube as before

## ADDED Requirements

### Requirement: Outdoor GI shading
Under an open sky the system SHALL shade the GI's light g with the openness raised to max(1 − (1 − openness)·a·32, openness) and b = (1 − a)²·0.08/(a + 0.035) + a, the light l = g × (level ambient × 15 × b × openness), and the ambient l × (the environment cube around the normal at mip 7 × 3 × 5). Reflections and the fog SHALL take l, with 3 in place of 8.

#### Scenario: Faint light outdoors
- **WHEN** the GI's light at a surface under an open sky has length 0.1 and openness 0.5
- **THEN** its lift is 0.81·0.08/0.135 + 0.1 and its openness is 1 − 0.5·0.1·32 held at 0.5, so 0.5

#### Scenario: Strong light outdoors
- **WHEN** the GI's light under an open sky has length 1 and openness 0.5
- **THEN** its lift is 1 and its openness stays 0.5

### Requirement: Open-sky environment cube
The system SHALL bind the open sky's environment cube (`genref02r`) to every lit surface of a level with an open sky, and the indoor cube (`genref01r`) otherwise, with or without voxel GI.

#### Scenario: Outdoor arena
- **WHEN** an outdoor arena loads after an indoor level
- **THEN** its surfaces and the materials reused from the indoor level light and reflect with `genref02r`

#### Scenario: Back indoors
- **WHEN** an indoor level loads after an outdoor arena
- **THEN** the reused materials bind `genref01r` again

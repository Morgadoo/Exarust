# Spec Delta

## MODIFIED Requirements

### Requirement: Fires
The system SHALL turn model nodes with a fire emitter into fires per the original's presets. A fire of size s up to 0.35 has one emitter at the node and a light of sRGB (1, 0.68, 0.43), intensity 8√s and radius 1500√s cm (×1.25 above 0.2, compared in double precision), 5 cm above the node. Its intensity is the emitter's density × (1 + 0.05 × a soft-bounded random walk) × that intensity, the density being the live particles over the template's scaled normaliser, smoothed by 0.8 and not clamped, so a torch's light runs above its template intensity on average, as the original's does.

#### Scenario: Menu torches
- **WHEN** the main menu scene is shown
- **THEN** its two torches light the walls around them in warm light

#### Scenario: A torch's light
- **WHEN** a torch's stored size 0.2 is turned into a fire
- **THEN** its light has intensity 3.58 and radius 838.5 cm

#### Scenario: A torch's average
- **WHEN** a torch's emitter runs for 20000 ticks
- **THEN** its density averages between 1.12 and 1.21 with 27.5 to 30.5 live particles, as the original's menu torches average 1.15 to 1.19 with about 29

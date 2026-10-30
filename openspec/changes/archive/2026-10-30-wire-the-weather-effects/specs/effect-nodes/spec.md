# Spec Delta

## MODIFIED Requirements

### Requirement: Rain colour
The rain colour SHALL be the environment's colour times 2.2, times 0.075·intensity + 0.075 (0.03·intensity + 0.03 while the game is paused), over 255, and its alpha (0.5·intensity + 0.5)·(|y| + 0.5).

#### Scenario: Full rain looking down
- **WHEN** the intensity is 1, the environment white and the backward axis's height −0.5
- **THEN** the red is 2.2·0.15/255 and the alpha 1

## ADDED Requirements

### Requirement: Live rain
While the scene's rain effect exists and the rain amount is above 0.01, the game SHALL step the rain on the game clock with the eye and the camera's backward axis, give it the scene's scratch colour, write its drift figure back as the rain sound's volume and its fall figure as the sound's pitch, and draw its streaks; the effect SHALL be freed with the scene's rain effect.

#### Scenario: Still gust at 0.9
- **WHEN** the rain at intensity 0.9 is updated with a gust of 0
- **THEN** the rain sound's volume becomes 0.1215 and its pitch 1.21

### Requirement: Rain streak vertices
The streaks SHALL be drawn as lines one window pixel wide (twice, one target pixel apart, on the 2× supersampled target) from the box turned 2° about its x axis and scaled by √(fov/70), with the drops' colour bytes read unnormalised (grey 160, alpha 0 at the head and 255 at the tail) times the rain colour, the alpha faded by (0.6 − 0.000667·d)⁴ with d the eye distance in cm.

#### Scenario: At the eye
- **WHEN** a tail vertex lies at the eye with a rain alpha of 1
- **THEN** its alpha is 255·0.1296

### Requirement: Rain streak fragment
Each streak fragment SHALL be the frame behind it t, clamped to 1, as 0.75·t + 3·max(t − 0.1, 0)² plus the vertex colour, clamped to 1 and blended with source alpha over one minus source alpha. A view without a copy of its frame SHALL draw no rain.

#### Scenario: Bright sky behind a streak
- **WHEN** a streak with colour 0.1 and alpha 0.5 lies over a frame value of 0.5
- **THEN** the fragment is 0.955 and the blend leaves 0.7275

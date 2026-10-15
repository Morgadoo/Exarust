## ADDED Requirements

### Requirement: Taps split over sub-frames
The system SHALL give each sub-frame of a supersampled frame two of the eight shadow-filter taps (sub-frame i takes taps 2i and 2i + 1, turned per window pixel), and SHALL split the GI lookup's taps the same way, as the original does.

#### Scenario: Supersampled pixel
- **WHEN** a window pixel is resolved from its four sub-frames
- **THEN** its shadow and GI values average all eight taps

### Requirement: Shadow-bias levels
The system SHALL give terrain and tile-grid floors shadow-bias level 3 and other grid set pieces level 1. A surface of level v > 0 SHALL compare shadow distances 0.995·k_c/k_r short, with k_r = 1 − 0.005·v and k_c = k_r − 0.01.

#### Scenario: Torch light grazing a floor
- **WHEN** a torch lights a terrain floor at a grazing angle
- **THEN** the floor shows no acne from its own shadow

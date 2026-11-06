## ADDED Requirements

### Requirement: Scripts and sounds on the frame
Sounds a fixed step starts and the scripts an AI's target pass reports SHALL be run by the frame after the step, in the step's order, with the step keeping only what it decides (which sound, which scripts); a voice that a later step fades SHALL be found by a key the step gave it.

#### Scenario: A throw that ends before its drop
- **WHEN** a throw's lift sound starts and the throw ends before its drop
- **THEN** the frame fades the lift's voice found by the key the step gave it

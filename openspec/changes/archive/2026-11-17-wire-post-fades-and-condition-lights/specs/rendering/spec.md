## MODIFIED Requirements

### Requirement: Post-chain fades
The screen fade SHALL run 5000 ticks with alpha `1 - (start + 5000 - now)/5000` in double precision, its bloom scale the base bloom strength times that alpha and its weight the alpha squared raised to the fifth. The desaturation fade SHALL run 3000 ticks with ramp `1 - (start + 3000 - now)/3000`; below a quarter the mask is `1 - 4 * ramp`, else `(ramp - 0.25) * 1.333`, and the pass receives the inverse; a non-zero kind adds `(1 - mask)^2 * 0.4` to the bloom.

#### Scenario: The end of the desaturation fade
- **WHEN** the ramp reaches 1
- **THEN** the mask is 0.75 * 1.333 = 0.99975 and the pass receives 0.00025

## ADDED Requirements

### Requirement: Run-out fades stop
A fade that has run out SHALL stop, the desaturation's kind with it; the desaturation fade SHALL be consulted only while no screen fade runs.

#### Scenario: A screen fade runs out
- **WHEN** 5001 ticks have passed since the screen fade started
- **THEN** its weight is 1 and the fade stops

### Requirement: Final pass fade vector
The final post pass SHALL mix the graded scene toward its bloom (less 0.03, at least 0) by the mask, scale it by the weight, and screen it with the bloom times 1.75 times the bloom scale plus the bloom term. Without a fade the vector SHALL be (1, the bloom strength, 0, 0), which leaves the picture as before.

#### Scenario: The first frame of a screen fade
- **WHEN** the screen fade starts this frame
- **THEN** the weight and the bloom scale are 0 and the view is black

### Requirement: Fade starts
The story start SHALL start the screen fade after its camera shot, once its level is in; the revive SHALL start it last. The mind awakening SHALL start the desaturation fade with kind 0 and the teleport bubble's hold with kind 1. The frame SHALL stamp each start with its clock.

#### Scenario: A new story
- **WHEN** a story starts and its level loads
- **THEN** the screen fades in from black over five seconds from the first frame the level is in

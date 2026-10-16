## ADDED Requirements

### Requirement: Blood jets
Each bleeding blow SHALL start one jet at once, also when its wound is dropped: at the contact point, riding with the struck body part, pointing against the striking point's swing. It SHALL send round(0.3 × s) droplets (halves to even) with s = 120·R, or 200 once R exceeds 1.67 in double precision, then stop. All jets SHALL share one clock that drops by 1 with probability ⅔ each tick and, while below 1, lets every running jet send a droplet and gains (83/40)·⅔ ticks.

#### Scenario: Chest cut
- **WHEN** the one-handed sword cuts a human's bare chest (R 0.457)
- **THEN** a jet of 16 droplets starts at the blow and sends them over about 0.4 s, at 40 a second

#### Scenario: Deepest wound
- **WHEN** a blow's wound exceeds R 1.67
- **THEN** its jet sends 60 droplets, over about 1.5 s

#### Scenario: No jet
- **WHEN** a skeleton is cut, or a human's hand at 4 cm a tick (R 0.045)
- **THEN** no jet starts

#### Scenario: Riding the limb
- **WHEN** the struck arm swings on while its jet runs
- **THEN** each new droplet starts where the arm now carries the contact point

### Requirement: Blood droplets
A droplet SHALL start at its jet with velocity v (cm/tick): the jet's frame applied to (±0.25, 0…1, ±0.25) uniform, × 48/83, + 0.15 of the jet's motion since its last droplet, then averaged with that droplet's (a new jet's: twice its axis in the struck part's frame); at 0.8·v ahead, its last position 1.2·v behind. It SHALL live 62 ticks, each tick aging and taking a Verlet step with a quarter of gravity, keeping 0.95 of its speed a second, touching nothing.

#### Scenario: Against the swing
- **WHEN** a blade swinging along +X strikes a part whose frame is the world's
- **THEN** the jet's first droplet leaves at 2 to 2.58 cm a tick along −X

#### Scenario: Falling
- **WHEN** a droplet at rest is left to fall
- **THEN** it drops 66.43 cm in the 61 ticks it moves and is gone on its 62nd, never moving sideways

#### Scenario: No floor blood
- **WHEN** droplets fall to the floor
- **THEN** they pass into it and leave no blood there

### Requirement: Drawn droplets
A droplet SHALL be drawn between the last two ticks as a camera-facing diamond along its motion less 0.25 cm/tick up, half-width s + ℓ·t²·24 (s 12–24 cm, fixed per ring slot; ℓ the sine of motion to view; t its age), stretched along by (2 − 2/(σ+1))·angle, alpha 0.5, 1, 0.5, 0.25, 0 at ages 0, ¼, ½, ¾, 1, a splat of `BldPrt02` under a square symmetry, multiplying the frame by the texel mixed to white by alpha, unlit, without depth writes or shadows.

#### Scenario: A slow droplet hangs down
- **WHEN** a new droplet at rest with half-size 24 cm is seen from the side
- **THEN** its diamond is 48 cm wide with its tips 39.08 cm above and below it

#### Scenario: Darkening only
- **WHEN** a jet sprays in front of the lit arena wall and against black
- **THEN** the wall shows dark red spatter where the splats fall, white parts leave it unchanged, and nothing shows against black

#### Scenario: Behind a body
- **WHEN** a body stands between the camera and the droplets
- **THEN** the droplets are hidden behind it

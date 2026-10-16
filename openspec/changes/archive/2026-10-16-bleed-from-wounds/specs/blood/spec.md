## ADDED Requirements

### Requirement: Bleeding blows
A blow SHALL bleed when the struck character's race bleeds and the wound's size R exceeds 0.05: R is the blow's lasting damage on the struck part, P/(P + 1) times the part's multiplier, before the red bar's 0.75. Blows on races that do not bleed, and blows with R at or under 0.05, SHALL leave no blood.

#### Scenario: Cut on a bare chest
- **WHEN** a human's bare chest takes the one-handed sword's cut at 20 cm a tick, 70 cm along the blade
- **THEN** R is 0.457 and the wound bleeds

#### Scenario: Threshold before the 0.75
- **WHEN** a human's bare hand takes the same sword's cut at 4 cm a tick (R 0.045), then at 6 cm a tick (R 0.064, red 0.048)
- **THEN** the first leaves no blood and the second bleeds

#### Scenario: Skeleton
- **WHEN** a skeleton (race flags 0) takes the chest cut
- **THEN** it loses health and shows no blood

### Requirement: Wound queue
A bleeding wound SHALL wait on the victim with the striking body (the blade, or the fist), the contact point as weights in that body's tetrahedron, the world point and R; at most seven SHALL wait, a new one dropped while seven wait. Each body tick SHALL count every waiting wound down from 7 and paint the first whose count is up, one a tick: 7 ticks (84 ms) after the blow, corpses included.

#### Scenario: Seven ticks
- **WHEN** a blow bleeds
- **THEN** nothing is painted for six body ticks and the wound is painted on the seventh

#### Scenario: Full queue
- **WHEN** nine bleeding blows land on one character in one frame
- **THEN** seven wait and are painted one a tick from the seventh tick to the thirteenth, and two are dropped

#### Scenario: Killing blow
- **WHEN** a bleeding blow kills its victim
- **THEN** its wound is painted on the corpse seven ticks later

### Requirement: Smear from the follow-through
A wound SHALL be painted with travel D, the striking body's struck point now (its weights in the body's tetrahedron as it is now) less the contact point, and radius 30·R + 10 cm, on the wound targets. Nothing SHALL paint the striker: a punch never bloodies the puncher.

#### Scenario: Blade swinging on
- **WHEN** the blade's struck point has moved 20 cm along +x by the seventh tick
- **THEN** the wound centres 10 cm behind the contact point, soaks the skin ahead along +x to the mesh's end, and behind is a ball of radius 30·R + 10 cm

#### Scenario: Radius
- **WHEN** wounds of R 0.05, 0.3 and 1 are painted
- **THEN** their radii are 11.5 cm, 19 cm and 40 cm

#### Scenario: Punch
- **WHEN** a fighter's fist strikes a bleeding blow
- **THEN** only the struck fighter is painted

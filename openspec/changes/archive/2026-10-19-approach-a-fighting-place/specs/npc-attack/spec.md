# Spec Delta

## ADDED Requirements

### Requirement: A fighter scores the approach
At 25 cm and at 200 cm, with reach 50 cm, size 1 and skill 0, the approach SHALL be bits 0x3ec00000. A wide fighter at 25 cm SHALL score bits 0x3f88b000. At 400 cm, and outside a fight, the approach SHALL be 0.

#### Scenario: Near, wide and past the band
- **WHEN** a fight with reach 50 cm, size 1 and skill 0 is scored at 25 cm, 200 cm and 400 cm, the same spot is wide at 25 cm, and 25 cm is scored outside a fight
- **THEN** the approach bits are 0x3ec00000 at 25 cm and at 200 cm, the wide score is bits 0x3f88b000, and 400 cm and the outside fight are 0

### Requirement: A recovery pulls the close term
A recovering swing at 25 cm, reach 50 cm and size 1, SHALL score close bits 0x403c0000 and approach bits 0x3fc00000.

#### Scenario: Recovering at 25 cm
- **WHEN** a recovering swing is scored at 25 cm with reach 50 cm and size 1
- **THEN** the close bits are 0x403c0000 and the approach bits are 0x3fc00000

### Requirement: A weapon hand publishes the arc
Action 5 at phase 0 SHALL publish kind 1, start bits 0x3fdf66f3, end bits 0xc01c61aa and look bits 0xbeb2b8c4. At phase 0.5 a hand on the side SHALL set the end to bits 0x3fc90fdb and keep that start. A recovering swing of that action SHALL publish kind 4 and keep that end.

#### Scenario: Early edge and a hand at phase 0.5
- **WHEN** action 5 is published at phase 0, again at phase 0.5 with a hand on the side, and again while recovering
- **THEN** phase 0 is kind 1 with start bits 0x3fdf66f3, end bits 0xc01c61aa and look bits 0xbeb2b8c4, phase 0.5 keeps that start and sets the end to bits 0x3fc90fdb, and the recovering publish is kind 4 with that end

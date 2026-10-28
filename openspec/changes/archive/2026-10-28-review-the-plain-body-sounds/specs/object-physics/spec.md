## ADDED Requirements

### Requirement: Plain body impact gate
A plain rigid body part's impact SHALL sound only when its strength is above 0.5 and either the clock has passed the part's cooldown or the strength is strictly above the stored threshold. A part without a collision sound SHALL park its cooldown at the all-ones word instead. A sounding impact SHALL store twice its strength as the threshold.

#### Scenario: A repeat at twice the strength
- **WHEN** a part that sounded at strength 10 takes 20 and then 21 before its cooldown
- **THEN** 20 is silent and 21 sounds

### Requirement: Plain body impact volume and pitch
A sounding impact SHALL play at volume min(strength·0.01, 1)·1.5, the minimum being the processor's MINSS. With a mobility of at least 0.1 or NaN it SHALL scale the pitch base min(…)·0.6 + 0.7 by mobility·0.02 + 0.9; with a lighter one it SHALL scale the volume by 0.8·(10·mobility) + 0.2 and the pitch base by 0.7·(10·mobility) + 0.3. The products SHALL be taken in double precision and narrowed once.

#### Scenario: A light part
- **WHEN** a part of mobility 0.05 takes an impact of 50
- **THEN** it plays at volume 0.45 and pitch 0.65

### Requirement: Plain body impact cooldown
A sounding impact's cooldown SHALL be now + 220 − round(170·base, half to even) + a 0–99 draw from the engine generator, which advances only when a sound plays.

#### Scenario: A first impact
- **WHEN** a part of mobility 0.3 with generator state 1 takes an impact of 50 at clock 1000
- **THEN** it plays at volume 0.75 and pitch 0.906 and its cooldown becomes 1138

### Requirement: Scrape move
Each tick, before the body's matrix is recorded, a scrape source SHALL place its point by its part's matrix from the last tick and take its squared move d in cm² since the last tick. A d over 0.01 SHALL become (√d·2 + d)·0.05, capped at 1, and the speed SHALL become 0.2·d + 0.8·speed in double. A body without sources SHALL skip the step.

#### Scenario: A 4 cm move
- **WHEN** a stopped source moves 4 cm
- **THEN** its move of 16 cm² is capped at 1 and its speed is 0.2

### Requirement: Scrape playback
A speed above 0.02 SHALL play at speed·volume and pitch·(0.1·speed + 0.9) at its part's position, starting a stopped source; otherwise, NaN included, a playing source SHALL stop.

#### Scenario: A 1 cm move
- **WHEN** a stopped source with volume 2 and pitch 1.2 moves 1 cm
- **THEN** its speed is 0.03 and it starts at volume 0.06 and pitch 1.0836

### Requirement: Stored sound slots
A registered prop body SHALL get one scrape source for each of its parts' stored sound slots with a sound name, at the slot's point, volume and pitch, starting where the point is so the first tick measures no move.

#### Scenario: A stored slot
- **WHEN** a prop body with one named slot registers
- **THEN** its body has one source on that part with the slot's point, volume and pitch

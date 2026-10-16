## ADDED Requirements

### Requirement: Swing pace
An attack's clock SHALL gain speed × g × 1.125 × base frames a tick, g = 1 − 0.833·m·(d + d²), d = |progress − 0.5|; a stab ×1.2, and below 40 % × (1.2·progress + 0.5); nothing under 0.8 of its most energy, (E/Emax)² under all of it. Speed, m and the recovery SHALL be the held weapon record's swing speed, swing shaping and stance blend. Attacks SHALL start with no blend, eased smoothly.

#### Scenario: Sword
- **WHEN** a sword of weight 0.3 and balance 0.6 swings
- **THEN** its clock gains 0.87 frames a tick at progress 0, 1.30 mid-swing and 0.97 at the end, and the attack ends into the stance with a blend of 14 frames

#### Scenario: Fists
- **WHEN** a fighter attacks unarmed
- **THEN** its clock gains 1.62 frames a tick throughout and the attack ends into the stance with no blend

#### Scenario: Duel
- **WHEN** `exarust --opponent play arena` runs
- **THEN** each of the duellist's attacks takes about 1 s from its start to its stance, and its strikes land

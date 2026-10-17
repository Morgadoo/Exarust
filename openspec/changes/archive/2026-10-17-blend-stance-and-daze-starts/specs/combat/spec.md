## ADDED Requirements

### Requirement: Dazed stance
While a fighter's energy is more than 0.05 under its most (in double precision), its stance SHALL be the Defence stance every body tick, and the player's stance rule SHALL not give Cmbt - Idle. Unless it is down or attacking, its clip SHALL start the Defence stance when another clip plays, with a blend of round((1.05 − E) × 300) frames, eased smoothly. A daze SHALL be decided before a fall in the same tick, so the fall's clip wins.

#### Scenario: Struck in the combat idle
- **WHEN** the player in combat mode, in Cmbt - Idle, is dazed to energy 0.62
- **THEN** its stance turns to the Defence stance, which starts with a blend of 129 frames, eased smoothly, and does not restart while the daze lasts

#### Scenario: Wits back
- **WHEN** the dazed player's energy is back within 0.05 of its most with its target near
- **THEN** its stance turns to Cmbt - Idle and the posture switch starts it with a blend of 30 frames

#### Scenario: Down or attacking
- **WHEN** a fallen or attacking fighter is dazed
- **THEN** its stance turns to the Defence stance and its clip plays on

#### Scenario: Duel
- **WHEN** `exarust --opponent --trace t.csv play arenao1` runs until the duellist kills the player
- **THEN** after each blow the player guards while dazed, and when its energy is back its targets turn under 0.5° a tick (10–18° in one tick without the daze's start)

#### Scenario: Shoved over
- **WHEN** `exarust --shove 60 --trace t.csv play arena` dazes the player to energy 0.13 and it falls
- **THEN** the Defence stance starts (blend 276), the fall replaces it, and the get-up ends in Idle without a jump

### Requirement: Stance blends
Each stance start SHALL blend from where the body is to the stance's first key: the set-stance helper (Tab, a duellist entering combat) 30 frames, smooth; the posture switch 30 frames, linear; a duellist's stance 40 frames without a primary striker, else twice its primary's stance blend, smooth, its ease set smooth on every pass even when nothing starts.

#### Scenario: Tab
- **WHEN** Tab turns combat mode off while the player plays the Defence stance
- **THEN** Idle starts with a blend of 30 frames, eased smoothly

#### Scenario: Posture switch
- **WHEN** a balanced fighter in Idle has the Defence stance
- **THEN** the Defence stance starts with a blend of 30 frames, linear

#### Scenario: Duellist
- **WHEN** a duellist holding a sword of stance blend 7, or with no primary striker, takes its stance from its combat walk
- **THEN** the stance starts with a blend of 14 frames, or 40, eased smoothly

### Requirement: Attack starts
An attack's clip SHALL start from where the body is, eased smoothly: the main side, the stab and the overhead with no blend past the first key, the other side with round((20 − W19) × 0.6) frames, W19 the stance blend of the striker it makes primary.

#### Scenario: Left straight
- **WHEN** the unarmed player throws an other-side attack
- **THEN** it starts with a blend of 12 frames, eased smoothly

#### Scenario: Main side
- **WHEN** a fighter throws a main-side attack
- **THEN** it starts with no blend, eased smoothly

## MODIFIED Requirements

### Requirement: Swing pace
An attack's clock SHALL gain speed × g × 1.125 × base frames a tick, g = 1 − 0.833·m·(d + d²), d = |progress − 0.5|; a stab ×1.2, and below 40 % × (1.2·progress + 0.5); nothing under 0.8 of its most energy, (E/Emax)² under all of it. Speed, m and the recovery SHALL be the swing speed, swing shaping and stance blend of the fighter's primary striker's record (its first held item before any).

#### Scenario: Sword
- **WHEN** a sword of weight 0.3 and balance 0.6 swings
- **THEN** its clock gains 0.87 frames a tick at progress 0, 1.30 mid-swing and 0.97 at the end, and the attack ends into the stance with a blend of 14 frames

#### Scenario: Fists
- **WHEN** a fighter attacks unarmed
- **THEN** its clock gains 1.62 frames a tick throughout and the attack ends into the stance with no blend

#### Scenario: Primary claw
- **WHEN** an unarmed fighter with a right fist and a left heavy claw throws an other-side attack
- **THEN** its swing is the claw's

#### Scenario: Duel
- **WHEN** `exarust --opponent play arenao1` runs
- **THEN** each of the duellist's attacks takes about 1 s from its start to its stance, and its strikes land

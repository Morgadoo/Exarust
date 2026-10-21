# Spec Delta

## ADDED Requirements

### Requirement: The stress flight
The stress SHALL be the stress base plus the stress less the courage, at least 0. With stress above 3, an alarm above 0.75 (0 for the contact already fled, its alarm raised to the one it fled at) and a way out, the contact SHALL be fled, scored by its alarm, before any other test.

#### Scenario: Stress 4 and alarm 0.8
- **WHEN** an alarm of 0.8 at stress 4 is classified with and without a way out, and the fled contact's alarm of 0.1 with the flight's alarm 2
- **THEN** the first flees scored 0.8, the second is noticed, and the fled contact keeps the flight scored 2

### Requirement: The engagement threshold
The threshold SHALL be `7b² + (2 + b)` for a tolerance b above 0 and `2 + b` otherwise; concern plus hostility above the threshold plus the stress, or above the threshold with an alarm above 1.5, SHALL engage at level 3, or 2 for an unreachable contact, scored by concern plus hostility.

#### Scenario: Thresholds
- **WHEN** concern 1 and hostility 1.5 are classified at tolerance 0, the same contact unreachable, concern 1 and hostility 3 at tolerance 0.5, then hostility 3.3, and hostility 1.6 at tolerance −0.5
- **THEN** they engage at level 3 scored 2.5, engage at level 2, are noticed, engage at level 3, and engage

### Requirement: Watching and noticing
Short of an engagement, a concern above 1 SHALL flee when it passes 8 less the stress and a way out exists, and SHALL watch, scored by the concern, when not; anything else SHALL be noticed with the score `6·(concern + hostility) + noticed − attention`, 3 more for the contact already attended.

#### Scenario: Concern 4.5 and a notice
- **WHEN** a concern of 4.5 at tolerance 1 is classified at stress 4 and at stress 0, and an attended contact of concern 0.5, hostility 0.25, noticed 1 and attention 0.25
- **THEN** the first flees, the second watches scored 4.5, and the notice scores 0.75·6 + 0.75 + 3

### Requirement: The contact list
A brain SHALL keep twelve contacts at most; with all twelve taken a new contact SHALL replace the one of least weight (ten times its sighting plus its concern, the first contact's sighting by size) unless it weighs less itself.

#### Scenario: A full list
- **WHEN** a full list of contacts weighing 5, one weighing 1, meets contacts weighing 2 and 0.5
- **THEN** the first replaces the light one and the second is not kept

### Requirement: The target pass's choice
The pass SHALL start from the foe it flees (at the alarm it fled at), or the foe it fights while concern plus hostility pass the raw stress plus 2. A higher level SHALL win, its current pick SHALL be renewed at any level, and at the same level the higher score SHALL win (the notice score kept apart at level 0). A flight SHALL flee the pick; nothing above a notice SHALL calm the brain toward the best noticed; anything else SHALL fight at the watch, or at level 3 for levels 2 and up.

#### Scenario: Notices, watches and a flight
- **WHEN** notices scored 2 and 3, watches scored 1 and 1.5, an engagement at level 2 and a flight are weighed in turn
- **THEN** the pass calms toward the notice scored 3, fights the watch scored 1.5, fights at level 3, and flees

### Requirement: The alarm mean and waking
The pass's alarm mean, its sum over the root of its count, doubled, SHALL raise the stress; a sleeper that finds anything above a notice SHALL wake.

#### Scenario: Alarm sum 3 over 4
- **WHEN** an alarm sum of 3 over 4 meets a stress of 1, and a sleeper finds a watch
- **THEN** the stress becomes 3 and the sleeper wakes

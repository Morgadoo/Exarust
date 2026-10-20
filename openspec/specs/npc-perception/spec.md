# npc-perception Specification

## Purpose
How an NPC rates one other character and chooses to notice, watch, engage or flee.

## Requirements

### Requirement: Hostility
With neutral traits, hostility toward a negative attitude SHALL settle near 0.0375 times that attitude's size. An attitude of −100 SHALL engage. An attitude of −20 SHALL only be noticed. Tolerance 100 SHALL keep the −100 contact from engaging.

#### Scenario: Demon and zombie
- **WHEN** hostility has settled for attitudes −100 and −20, and the −100 contact is given tolerance 100
- **THEN** the first engages, the second is noticed, and the tolerant one is noticed

### Requirement: Choice
Concern above 1, with no heat past the threshold, SHALL watch. Stress above 3 and alarm above 0.75, with a flee route, SHALL flee.

#### Scenario: Watch or flee
- **WHEN** concern is 1.5, then stress is 4 and alarm is 0.8 with a flee route
- **THEN** the first choice is watch and the second is flee

### Requirement: Range and view
Closeness at the observer SHALL be 0.99. It SHALL be 0 at 12 m and beyond, unless the contact is already tracked, which extends the range to 18 m. Asleep detection SHALL be 6 m. A hidden contact in no light SHALL be seen only to 1.2 m. The view weight SHALL be 1 straight ahead and 0 straight behind.

#### Scenario: Ahead and out of range
- **WHEN** the other stands on the observer, then 12 m away, then straight behind
- **THEN** closeness is 0.99, then 0, and the view weight is 1 ahead and 0 behind

### Requirement: The duellist engages
A duellist whose contact has settled on engage SHALL take the guard.

#### Scenario: Hostile contact
- **WHEN** a duellist rates a contact whose settled choice is engage
- **THEN** it takes the defence stance

### Requirement: Personality from the record
The skill share SHALL be the combat skill over 100, capped at 1. With t1 and t2 the record's second and third traits, the confidence SHALL be t1/100, the restraint −t2/100, the courage (confidence + 1)/2, the eagerness ((3 − restraint) + confidence + 3)/8, and the blend `(1 − h) + h·(confidence + 2)·0.333` with h = (1 − feral)², or 1 for a marked body. The jitter SHALL be the square of a draw spread over −1..1, and the arousal SHALL start at 10.

#### Scenario: A half-feral record
- **WHEN** a record of combat skill 150, traits 20 and −30 and feral 0.5 is read with the generator at 0
- **THEN** the skill is 1, the confidence 0.2, the restraint 0.3, the courage 0.6, the eagerness 5.9/8, the blend 0.75 + 0.25·2.2·0.333 and the jitter about 1

### Requirement: Attaching the brain
Without the race word's AI flag no contact sense SHALL be installed. With it, the sense SHALL be the ally sense for race class `0x12xx` and the standard one otherwise; a dead character SHALL get the effect at 0 and its roles SHALL NOT be applied; a statue race SHALL stand sentry with the effect at 0; any other SHALL get the effect at 1.

#### Scenario: Ally, dead and statue
- **WHEN** brains are attached to an ally-class body, a dead body and a statue body, all with the AI flag
- **THEN** the first has the ally sense and the effect at 1, the second the effect at 0 and no roles applied, and the third is a statue with the effect at 0

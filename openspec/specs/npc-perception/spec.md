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

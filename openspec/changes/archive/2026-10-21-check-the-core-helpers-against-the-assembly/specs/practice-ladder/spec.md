# Spec Delta

## ADDED Requirements

### Requirement: Practice opponent and game tips
The practice opponent SHALL be the partner's template read without the cache, with its first three personality traits −100, 100 and 100, on team 0x10, waiting 3000 ms after the fight starts. The tips popup SHALL show, for index 0 and with its setting on, the tip whose index is the draw over the least-shown tips' count (as the original, not the draw-th least-shown tip), counting a zero up to two; index n from 1 SHALL show control hint n − 1.

#### Scenario: Repeated tips
- **WHEN** tips are shown twice from a fresh popup and generator
- **THEN** tip 0 is shown both times and its count is 3

#### Scenario: The viewer's opponent
- **WHEN** `exarust play arena --opponent` runs with the actors installed
- **THEN** the duellist is dressed from opponent01, sits on team 0x10 and waits three seconds before it engages

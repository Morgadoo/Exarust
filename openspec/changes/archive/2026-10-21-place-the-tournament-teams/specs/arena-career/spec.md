## ADDED Requirements

### Requirement: Tournament teams
A tournament event SHALL group its entry slots in runs of the team size: from the first slot holding an id, the team SHALL be the run it falls in, its id the first held id and its members the slots with a positive id; the next team SHALL start after the run, until the event's team count is formed or the slots end.

#### Scenario: Two teams of two slots
- **WHEN** slots 1, 1, 0, 5, 0, 0, 7, 7 form two teams of two slots, slots 0, 0 form any, and a team size of 0 is asked
- **THEN** team 1 holds slots 0 and 1, team 5 holds slot 1, team 7 is cut off, and the last two form none

### Requirement: Tournament ring
The ring's radius SHALL be the teams times the team size times 50 plus 100, at most 350. Team `g` of `G` with `n` members SHALL spread them over `2 (n − 1)/(n + 1)` (in double, stored as single) from `g/G × 6` less half the spread, one every spread over `n − 1`. A member at `a` SHALL face the direction of the turn times `(3 − a) × 0.1666666` in double and stand the radius against it.

#### Scenario: A lone member and a full ring
- **WHEN** the two teams above are placed, and two teams of three slots
- **THEN** the radius is 300, the lone member stands at (0, 0, −300) facing +z, the first member sits at a = −1/3, and the larger event stops at a radius of 350

### Requirement: Tournament opponents
A team id under 2 SHALL bring a generic fighter of level 5 at the event's tier, any other the company's fighter; a team SHALL be hostile while at most two teams fight, else every team SHALL fight for itself.

#### Scenario: Generic and company
- **WHEN** team 1 of two and team 6 of three are asked at tier 3
- **THEN** team 1 brings a level-5 fighter hostile on team 1, and team 6 a company fighter in a free for all

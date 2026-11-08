## ADDED Requirements

### Requirement: Company match copies
A match start SHALL make each hired company entrant's body from a match copy of its record: its six skills, a fighting skill of ten for each level plus ten under ten levels and 100 from ten levels, and the loadout of its rank (half its levels) capped by the match's tier, or two under the tier for a Captain's entrants after the first.

#### Scenario: Rank capped by the tier
- **WHEN** a fighter of seven levels enters a match of tier 1
- **THEN** its body wears its rank-1 loadout and fights at skill 80

#### Scenario: Captain's wing
- **WHEN** the same fighter is the second entrant of a Captain at tier 4
- **THEN** its body wears its rank-2 loadout

### Requirement: Match copy hands
A match with a tier below 0 SHALL put on the first loadout with the item of every hand emptied, keeping each hand's flags. A weapon event SHALL write the started event's first entrant's first weapon word into the main hand and its second word, the item cleared, into the off hand. The copy SHALL carry no inventory and no loadouts and SHALL put on underwear when nothing worn covers the groin.

#### Scenario: No tier
- **WHEN** it enters a match of tier −1
- **THEN** its body wears the first loadout's worn items with all four hands empty

### Requirement: Tournament event start and forfeit
A tournament event's start SHALL give each earlier event that has no winner and no entrant a rival winner of 2 to 7, stopping at the first entered event. The board's Forfeit SHALL ask "Do you wish to forfeit the entire tourney?" and on Yes empty every event that has no winner. The end screen's Continue and the forfeit SHALL first resolve every company body under 0.01 health whose fall the match tick has not resolved yet.

#### Scenario: Earlier events settled
- **WHEN** the company starts the second event and did not enter the first
- **THEN** the first event's winner is a rival from 2 to 7 and the later events stay open

#### Scenario: Forfeit
- **WHEN** the board's Forfeit is answered Yes
- **THEN** every event without a winner loses its entrants and a decided event keeps them

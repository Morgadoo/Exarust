## ADDED Requirements

### Requirement: Arena round transition
A round's end SHALL set the match state (8 next fight, 5 lost, 4 won) and open transition screen 3 while the state is the next fight, else screen 2, and SHALL award experience either way.

#### Scenario: Elimination rounds
- **WHEN** an Elimination match wins four rounds, the round counter rising after each, and then wins a fifth
- **THEN** the first four open screen 3 with state 8 and the fifth opens screen 2 with state 4

### Requirement: Arena end screen Continue
The end screen's Continue SHALL send a dead manager to the main menu, SHALL start another round while the state is the next fight and the round less the losses is below the extra rounds (a tournament raising the bracket's next fight), and SHALL otherwise settle the match and return to the hub, saving the manager at 300, a half turn and flag 0x100.

#### Scenario: Continue branches
- **WHEN** an Elimination match at round 3 continues in the next-fight state outside and inside a tournament, and at round 4
- **THEN** round 3 starts round 4 (the tournament with the bracket), and round 4 returns to the hub

### Requirement: Arena next round bodies
The next round SHALL keep the company's fighter and generate a fresh opponent at the match's level and gear tier, unless the last round was lost and the fighter is dead; then the entrant at the place of the losses SHALL come in against the opponent who won, and a fresh opponent SHALL be generated only when no opponent body is left.

#### Scenario: Reserve entrant
- **WHEN** the second loss leaves the fighter dead and the opponent standing
- **THEN** reserve entrant 2 comes in, the opponent is kept and no opponent is generated

### Requirement: Arena dead-body pass
Before the end screen and the return, every company body whose health widened to double is below 0.01 and whose fall is not yet resolved SHALL be resolved once; NaN health SHALL never be resolved.

#### Scenario: Single 0.01
- **WHEN** a company body has health 0.01 in single precision and a physician is employed
- **THEN** its fall is resolved and logged as saved

### Requirement: Arena match entry fee
The match start SHALL subtract the event's entry fee from the purse, stopping at 0, and add the whole fee to the spending; a short purse SHALL not refuse the match.

#### Scenario: Short purse
- **WHEN** a Duel with a 3 copper fee starts with 2 copper
- **THEN** the match launches, the purse is 0 and the spending rose by 3

### Requirement: Tournament prize history
The weekly prize draw SHALL refuse a prize found in any of the eight history slots, then rotate the history and store the prize first.

#### Scenario: Eighth slot
- **WHEN** seed 13 first draws prize 8 with the history 1 to 8
- **THEN** that draw is refused and a prize outside the history is stored first

### Requirement: Arena roster health gauge
The roster's health gauge SHALL keep the cell's left, take the top less six units plus half of five units (rounded down) plus one, the width less one unit and a thickness of two units, filled by the record's word times the double 1/65535 stored as single.

#### Scenario: Base unit
- **WHEN** a cell at (100, 50) of width 60 draws a full word at unit 4
- **THEN** the gauge is at (100, 37), 56 long and 8 thick, filled to 1

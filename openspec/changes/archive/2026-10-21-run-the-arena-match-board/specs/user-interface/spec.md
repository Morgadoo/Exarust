## ADDED Requirements

### Requirement: Match board selected match
The arena match board SHALL build its lineup for one selected match record
copied by the open tab: a standard match on the standard tab, the selected
daily event on the events tab and the selected tournament event on the
tournament tab. The record's lowest rank SHALL be the rank floor, its side
size SHALL cap the lineup, its kind SHALL select the Captain rule, flag 1
SHALL lift the one-leader and marked-fighter rules and flag 4 SHALL mark a
weapon event.

#### Scenario: A daily Captain event drives the lineup
- **WHEN** the events tab selects a Captain event with floor 3 and side
  size 2, and the company's ranks are 2, 1, 0 and 4 with the last a leader
  and the player at rank 3
- **THEN** the lineup holds only the rank-4 leader, because the first
  fighter needs the full floor and the player would be a second leader

### Requirement: Match board standard matches
The standard tab SHALL list Duel, Doubles, Skirmish, Fray, Reserve and
Pugilism with side sizes 1, 2, 3, 1, 3 and 1. It SHALL multiply the
match's fee and prize by the rank plus one through the original's coin
packing, keep the tier at or below the rank, move the tier to the rank's
row on a rank change, grey the tier rows above the rank, and store no tier
for Pugilism.

#### Scenario: A Duel at Novice
- **WHEN** the Duel is selected at rank Novice with tier Master
- **THEN** the tier becomes Novice, the rows Adept, Expert and Master are
  greyed, the floor is Novice, the fee is 9 copper and the prize 3 silver

#### Scenario: Pugilism at Expert
- **WHEN** Pugilism is selected and the rank changes to Expert
- **THEN** no tier is stored, the fee is 5 copper and the prize 2 silver
  and 6 copper

### Requirement: Match board lineup rebuild
The lineup SHALL be rebuilt by walking the lineup order and adding each
combatant whose rank passes the floor until the side size, with at most one
leader, the player counting as one, unless flag 1 is set.

#### Scenario: The player is the second leader
- **WHEN** a six-a-side match with floor Inept rebuilds over ranks 2, 1
  (leader), 0 and 4 (leader), with the player last in the order
- **THEN** the lineup is the first three and the player stays in the
  company table

### Requirement: Match board fighter moves
A double click SHALL add a fighter when there is room and otherwise bench
the last teammate to the end of the company table; a drop on a row SHALL
swap with it; a marked fighter SHALL swap with the first marked teammate. The
fighter's place in the lineup order SHALL become the row it joined, or the
first place after a marked swap or a bench. A fighter moved back SHALL be
inserted in pool-slot order.

#### Scenario: A bench puts the newcomer first in the order
- **WHEN** a two-a-side lineup of slots 1 and 2 is full and the player is
  double-clicked
- **THEN** slot 2 is benched to the end of the company table, the lineup is
  1 and the player, and the next rebuild puts the player first

### Requirement: Match board Captain moves
On a Captain event with more than two teammates of whom fewer than two
reach the floor, a fighter under the floor SHALL NOT replace one who does,
and after a move a qualified teammate SHALL be moved first.

#### Scenario: An unranked Captain newcomer
- **WHEN** a Captain lineup of ranks 1, 2 and 4 with floor 3 receives a
  rank-0 fighter dropped on the rank-4 row
- **THEN** the drop is refused, and a drop on the rank-1 row is taken and
  moves the rank-4 teammate first

### Requirement: Match board tournament status
The tournament tab SHALL show the six events' winners, reading a winner of
1 as the player's company, below 1 as a dash, and from 2 through the
tournament's company table. Its status SHALL say "The tournament has
ended." when every entered event is settled and the last event has a
winner, and "No more events entered." when it has none. On a tournament day
with entries the board SHALL switch to this tab and lock fighter assignment.

#### Scenario: The last event decides the end
- **WHEN** the only entered event has a winner and the last event has none
- **THEN** the status reads "No more events entered.", and once the last
  event has a winner it reads "The tournament has ended."

### Requirement: Match board weapon event
A weapon event SHALL show the weapon row's name on the reward panel,
shorten the lineup to 120 pixels and mark the panel when the first entrant
does not carry one of the row's weapons.

#### Scenario: The weapon check
- **WHEN** the selected tournament event names the row "Mace" with weapons
  7 and 9 and its first entrant carries 9, then 8
- **THEN** the panel reads "Mace" and is unmarked, then marked

### Requirement: Match board day controls
The standard and events refreshes SHALL write the no-more-matches message
once three matches were entered that day and clear it otherwise, and the
tournament refresh SHALL clear it. After each rebuild or move the primary
button SHALL be disabled once three matches were entered or while the
lineup is short, and hidden on the season tab. A zero coin cell SHALL be
drawn as a dash four pixels right.

#### Scenario: The day's matches are spent
- **WHEN** three matches were entered and the standard tab refreshes
- **THEN** the message box shows the no-more-matches text and the primary
  button is disabled

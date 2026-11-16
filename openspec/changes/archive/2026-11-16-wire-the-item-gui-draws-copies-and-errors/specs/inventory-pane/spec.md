## MODIFIED Requirements

### Requirement: Place pass after a centre add
Every item added at a panel's centre SHALL then take a random spot in the middle half of the panel and be pushed out of the other entries, drawing x then y from the frame's engine generator; a carried copy SHALL take the cell it ends at.

#### Scenario: A crystal given back
- **WHEN** a socket's crystal goes back into the open pack with the frame's engine generator at 0
- **THEN** its entry is not at the panel's centre, its carried copy has its cell, and the generator stands two steps on

## ADDED Requirements

### Requirement: Equipment scatter draws
A looted body's equipment scatter SHALL draw each item's spot from the frame's engine generator and SHALL first store 0xBA28 as the run-time library's seed, so the frame's run-time stream starts again from that seed.

#### Scenario: One worn item
- **WHEN** a body wearing one item is looted with the generator at 0
- **THEN** the generator stands two steps on and the run-time stream is the one seeded with 0xBA28

### Requirement: Card sentence draws
The description card's sentences SHALL draw from the frame's engine generator, each card going on from where the last one left it.

#### Scenario: Two cards
- **WHEN** two described items' cards are shown in turn
- **THEN** the generator holds after each what the sentence builder leaves for that card from the state before it

### Requirement: Local copy on a drop
A drop into an item panel from a panel other than its linked one SHALL make a level-local copy of an untagged base record whose header word has bit 0; the destination SHALL list the copy with its own icon and the payload's flags, the source SHALL give up the base id, and any other id SHALL move as it is.

#### Scenario: A marked and an unmarked item
- **WHEN** a marked base item and an unmarked one are dragged from the pack into a container popup
- **THEN** the popup lists the local copy then the unmarked id, and the local tier holds one record

### Requirement: Socket row drag-over
The socket row SHALL take a drop only of the four power crystals; any other item SHALL get the control's nothing answer, leaving the item's sockets unread and unchanged.

#### Scenario: Not a crystal
- **WHEN** a pack item that is no crystal is released over a socket of the shown card
- **THEN** the socket words are unchanged and the item stays in the pack

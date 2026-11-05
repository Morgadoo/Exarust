# Spec Delta

## ADDED Requirements

### Requirement: Crystal drop on a card socket
A release over an existing socket of the shown item card's socket row SHALL be the socket control's drop. Only the four power crystals SHALL be taken, anything else refused and the drag cancelled. The socket words SHALL be read from the item first; the dropped crystal SHALL take the socket's kind and the drag's charge (bits 16–23 of its flags), and the source's drag SHALL end as accepted, so the crystal leaves its pack.

#### Scenario: Crystal C into the flame sword
- **WHEN** `exarust grid "play exanima01 --ability-item 3,cold --give 452 --press I@100 --item-card 0x487@150 --click-at …@170 --drag-at <crystal>><socket>@190-220 …"` runs
- **THEN** the log shows the crystal leaving the pack into socket 0 (word 0x0084), the card draws it, and the next ability check lights the blade

### Requirement: Crystal given back and the row committed
A crystal already in the socket a crystal is dropped on SHALL go back to the player: into the open pack at its centre, else onto the carried list. After a drop or a drag out the row SHALL be committed into the item's weapon section (byte 0x48), else its apparel body (byte 0x218).

#### Scenario: Swapping crystals
- **WHEN** crystal A is dropped on a socket holding crystal C with charge 0x12
- **THEN** the socket holds A and crystal C with flags 0x120000 is back in the player's pack

### Requirement: Crystal drag out of a card socket
A press on a socket of the shown card's row that holds a crystal SHALL start dragging the crystal with its charge, the socket keeping it until the drag ends. A release on another socket of the row SHALL drop it there; one on a pack panel SHALL put it there. Either way the socket it left SHALL keep only its bare mark (the charge byte stays) and the row SHALL be committed. A release anywhere else SHALL cancel and leave the socket as it was.

#### Scenario: Taking the crystal back
- **WHEN** the crystal in the lit flame sword's socket is dragged onto the player's pack
- **THEN** it is back in the pack, the socket word is 0x0080, and the next check puts the blade out

# Spec Delta

## ADDED Requirements

### Requirement: Loadout mode records
The loadout form SHALL change its control records by mode word before layout: mode 1 SHALL widen Previous and Next to 112 with Next at x 120, set records 4 to 9 and 11 to 20 to the hidden state 0x110 and records 22 to 27 to 0; mode 3 SHALL set records 1, 6 and 11 to 14 to the disabled state 0x10 and records 2, 3, 4 and 7 to 0x110; other modes SHALL keep the shipped table.

#### Scenario: Mode 1 shows the hire block
- **WHEN** the form's records are built for mode word 1
- **THEN** Dismiss and the shelf buttons read 0x110, CLOSE keeps its word and the HIRE record reads 0

### Requirement: Loadout equipped marks
On each refresh the loadout form SHALL clear bit 0x200 from every arsenal entry and set it on each entry whose item is in either hand or the worn list of the shown record.

#### Scenario: A worn item is marked
- **WHEN** the shown record holds item 2 in a hand and wears item 3, and entry 1 carries 0x400
- **THEN** entries 2 and 3 carry 0x200 and entry 1 keeps only 0x400

### Requirement: Divination card sides
The divination card draw SHALL paint both the back and the face of a turning card, back first, with face culling off.

#### Scenario: A card past edge-on
- **WHEN** a card turned by 2 radians is painted
- **THEN** its shadow, back and face layers are all drawn

### Requirement: Tip banner text
The tips popup SHALL show tip k from the eighteen-entry tip table for an automatic draw and, for index n from 1 to 6, the indexed message n − 1 (the control hint, the defeat, the death and the three victory notices), and SHALL be hidden when the end menu hides or the prelude closes, keeping the shown counts.

#### Scenario: The death notice and its hide
- **WHEN** index 3 is shown and the prelude then closes
- **THEN** the banner reads "You have died." and is then empty, the counts unchanged

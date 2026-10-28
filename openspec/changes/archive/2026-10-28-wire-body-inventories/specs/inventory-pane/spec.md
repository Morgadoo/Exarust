## ADDED Requirements

### Requirement: Body loot panels
When a body's loot window opens, its pack SHALL hold the body's carried items at their saved cells (sixteenths of a GUI unit scaled by the unit) and its equipment panel SHALL hold the worn items in wearing order followed by each item of the set in the hands that has a shield section; held weapons and empty slots SHALL be left out.

#### Scenario: A record's panels
- **WHEN** a body carries 0x30 at (40, 4) and 0x31 at (8, 16), wears 0xa0 and 0xa1, and holds 0x11 and the shield 0x12, at GUI unit 4
- **THEN** the pack holds 0x30 at (10, 1) and 0x31 at (2, 4), and the equipment panel lists 0xa0, 0xa1 and 0x12

#### Scenario: A real corpse
- **WHEN** each dead body of exanima01 with carried items has its loot window bound
- **THEN** its pack lists the record's carried items in order with their flags

### Requirement: Body item transfer
A drag from a body's panel dropped on another pack SHALL move the item from the body's possessions into the destination's carried list at the drop's cell, removing a worn item, a hand item with its held carried copy, or the last carried copy; a drop on the same panel SHALL only reposition the entry, and a drop on an equipment panel or outside every panel SHALL be refused.

#### Scenario: Taking off a worn item
- **WHEN** the worn item 0xa0 is moved to the player's pack at cell (40, 60) with unit 4
- **THEN** the body wears only 0xa1 and the player carries 0xa0 saved at (160, 240)

### Requirement: Loot window flags
The loot window's show rule SHALL read the body's record profession word and refuse a body whose race byte is 3, and the deferred close SHALL close a window whose body's condition is no longer 0xa00.

#### Scenario: Race three
- **WHEN** a loot window is requested for a dead body of race 3
- **THEN** no window opens

#### Scenario: Revived body
- **WHEN** a body's loot window is open and its condition changes from 0xa00 to 0
- **THEN** the window closes

# Spec Delta

## ADDED Requirements

### Requirement: Loot window
The loot window SHALL be titled "Body", 244 by 440 at x 400 with an unset y, with a 228 by 180 pack panel at its client's bottom and an equipment panel above it ending four units before the pack. It SHALL be refused for a character whose flag word's second byte is 3, activate an open window for the same body, and otherwise open a new window per body registered as an open container under the body's id with 0x20000000.

#### Scenario: Two bodies
- **WHEN** a body's loot window is open and another body is looted
- **THEN** a second window opens and the first stays

### Requirement: Right-release close
A right-button click release over a loot window or the inventory SHALL close it; the journal SHALL NOT close that way.

#### Scenario: Release over the inventory
- **WHEN** the right button is released over the focused inventory
- **THEN** the inventory closes

### Requirement: Power slot draw
A filled power slot SHALL draw its frame inset by the border and centre a 32-pixel icon; under the pointer the icon SHALL take the full-scale draw with no ink, otherwise the bright ink for state bit 0x10, the marked ink for bit 0x40, the bright ink for kind 2, else the plain ink. An empty slot SHALL draw a shadow and its kind icon only for kind 4, or for a kind past 2 under the pointer.

#### Scenario: Kind 3 empty slot
- **WHEN** an empty kind-3 slot is not under the pointer
- **THEN** no kind icon is drawn

### Requirement: Inventory preview build and bars
The preview's build callback SHALL always answer done and, when an open inventory shows the view and the view holds a character, set the inventory camera to ten times the character's height. The preview bars SHALL start 50 units left of the half width and 5 units above the bottom, 48 units wide and 52 apart, framed 2 pixels out and 4 units minus 2 down.

#### Scenario: Camera refit
- **WHEN** the build runs for a 1.7-high character shown in an inventory
- **THEN** the camera distance is 17

### Requirement: Selector forget row byte
The technique selector SHALL mark the row at each chosen technique byte minus one without a bound check below ten, so a byte of 9 marks the forget row before a positive count clears it.

#### Scenario: Byte nine
- **WHEN** a chosen technique byte is 9 and the count is 0
- **THEN** the ninth row is marked

### Requirement: Form tree layout and control messages
Showing a form SHALL resolve every unset place in its tree depth first with the per-axis rules (centred, right or bottom anchored minus 4, else 4 and 36). A message sent by control id SHALL reach the first record with that id in the tree, own records before children, with its code and two words; without one it SHALL be ignored.

#### Scenario: Missing id
- **WHEN** a message is sent to an id no record has
- **THEN** nothing answers

### Requirement: HUD reset and status values
The HUD reset SHALL take one off the GUI hold count only when the level curtain has fallen, clear the curtain and start a running heartbeat's fade. The status glow SHALL use radius 8 and 0.6 of the pulse, the status icon the tint (140, 160, 255), and a hide clock SHALL be the delay added to the clock, wrapping.

#### Scenario: Curtain not fallen
- **WHEN** the HUD resets while the curtain is still falling
- **THEN** the hold count is unchanged

## ADDED Requirements

### Requirement: Placement marker overlap refusal
A placement marker that matches the held item SHALL be refused when the item, moved so its held part takes the marker's frame, would overlap another body; the marker search SHALL then go on with the next marker, and the item SHALL take the first marker it does not overlap.

#### Scenario: Overlapped first marker
- **WHEN** two markers match the item and it would overlap a body at the first
- **THEN** the item snaps onto the second marker

### Requirement: Use cursor begin and end
The use cursor's begin SHALL open the GUI item named by an operative section of class 0 and kind 0x11, and otherwise SHALL lift the cursor only while play is on (state word 0x100, cursor mode 0x40). Its end SHALL answer 0x81 to event 0x80 only, run the use only when the release is over the world, set the player's action word to 4, clear the use state and restore cursor mode 4; any other event SHALL answer 1.

#### Scenario: Release over the world
- **WHEN** the right button's release (0x80) arrives over the world
- **THEN** the use runs and the callback answers 0x81

### Requirement: Deferred drop word gates
The deferred drop SHALL free a queued node when the controls' state word lacks 0x1000, SHALL report a node without physics as a content error and reset the word to 0x1000, and SHALL otherwise place the node with the drag's condition word and the −100000 cm limit, copying the body's frame into the controls only when the word lacked 0x2000, and end with the word at 0x2000.

#### Scenario: Already placed once
- **WHEN** a node with physics runs with the word 0x3000
- **THEN** it is placed, its frame is not copied and the word becomes 0x2000

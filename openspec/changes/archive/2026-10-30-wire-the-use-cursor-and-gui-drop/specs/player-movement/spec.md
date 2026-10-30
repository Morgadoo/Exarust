# Spec Delta

## MODIFIED Requirements

### Requirement: Use cursor begin and end
The use cursor's begin SHALL open the GUI item named by an operative section of class 0 and kind 0x11, and otherwise SHALL lift the cursor only while play is on (state word 0x100, cursor mode 0x40). Its end SHALL answer 0x81 to event 0x80 only, run the use only when the desk's button word has bit 0, set the player's action word to 4, clear the use state and restore cursor mode 4; any other event SHALL answer 1.

#### Scenario: Release over the world
- **WHEN** event 0x80 arrives with the desk word's bit 0 set
- **THEN** the use runs and the callback answers 0x81

## ADDED Requirements

### Requirement: Panel double-click class
An item panel's double click on an entry SHALL call the use hook when the entry's double-click class is 2: no usable section, classes 2, 4 and 8, and a class-0 kind other than 0x11 SHALL be class 0; a class-0 kind 0x11 SHALL be 2 while the entry's 0xFF0000 bits are clear; a class-1 liquid or kit (kinds 8, 14, 9, 10) with those bits set SHALL be 1; anything else SHALL be 2.

#### Scenario: A used salve
- **WHEN** a healing salve whose entry carries 0xC80000 is double-clicked
- **THEN** its class is 1 and only the card closes

### Requirement: Live use cursor end
While the cursor is up, the next press of any mouse button SHALL end it and SHALL not reach the hand, the item panels or the card; a left press SHALL run the dispatcher, which unlocks a latched door whose key is the used item.

#### Scenario: A key used and put down
- **WHEN** a key's use hook lifts the cursor and the right button is pressed
- **THEN** the cursor ends without a use and the press reaches nothing else

### Requirement: GUI drop into the world
A drag out of the player's pack held over the world view, with no drop pending and nothing on the item cursor, SHALL be taken by the controls; on the next frame the item SHALL be built with its bodies, and SHALL be reported as having no physics (the drag cancelled) when it has none, or SHALL be put where the cursor's ray meets the level, leave the pack, and be lifted onto the item cursor once its body exists while the button stays down.

#### Scenario: A salve with no model
- **WHEN** a claimed drop's item builds no body
- **THEN** the controls' word is 0x1000 again, the claimed drag is gone and the queue is empty

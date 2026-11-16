## ADDED Requirements

### Requirement: Root task key slot
The root task's key slot SHALL ignore a key whose word has either low bit and Shift alone, and otherwise SHALL run the one shortcut for the key whose required modifiers are all held and most numerous, the earlier entry on a tie; it SHALL pass nothing on.

#### Scenario: Two bindings for one key
- **WHEN** Ctrl+S and a second Ctrl+S binding exist and S goes down with Ctrl held
- **THEN** only the first binding's lines run

### Requirement: Root task table
The root task's shortcuts SHALL be the settings file's bound `KeyDef` lines in file order, then Escape with no modifier as the main menu's toggle.

#### Scenario: Escape with Ctrl held
- **WHEN** Escape reaches the root task with Ctrl held
- **THEN** the menu toggle runs

### Requirement: Keys that climb to the task
A key that a game screen's or the main menu's or the settings form's key slot passes up SHALL reach the root task's key slot after every form's handler of the frame; with no screen or form holding the keys, the game view SHALL pass each key-down on after the player's hotkeys.

#### Scenario: A binding over the view
- **WHEN** the pointer is over the game view and Ctrl+S goes down
- **THEN** the hotkeys hear S and the Ctrl+S binding runs

### Requirement: Escapes Exarust keeps
Escape over an open game screen and on the settings form SHALL stay Exarust's close of that screen or form and SHALL NOT reach the root task.

#### Scenario: Escape over the inventory
- **WHEN** the inventory is open and Escape goes down
- **THEN** the inventory closes and the menu toggle does not run

### Requirement: Main menu key slot
The main menu's key slot SHALL shift each typed character into a ten-byte buffer, A-Z lowered, and toggle the menu music when it reads `barechoons`; a key-down SHALL go to the base form's key slot and up to the task.

#### Scenario: The music code
- **WHEN** BareChoons is typed on the open menu
- **THEN** the alternative-music flag sets and the drum-and-bass theme plays at 0.2

### Requirement: Menu toggle on the open menu
The root task's menu toggle SHALL go back on the open menu's start and arena pages and close the menu elsewhere; with the menu closed it SHALL leave the opening to F1 and the screens' own Escape.

#### Scenario: Escape on the start page
- **WHEN** the menu shows the start page and Escape goes down
- **THEN** the title page shows, and a second Escape closes the menu

### Requirement: Hub fields from the window procedure's keys
The hub's focused text field SHALL take each key-down's edit key with Ctrl and Shift as the key-down's word holds them, then the character it typed, in the window procedure's order.

#### Scenario: A capital after a move
- **WHEN** a, Shift+B, Left and c go down over the emblem name
- **THEN** the name gains acB

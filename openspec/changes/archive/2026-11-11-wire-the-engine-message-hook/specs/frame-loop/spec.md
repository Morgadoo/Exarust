## ADDED Requirements

### Requirement: Message hook log file
Every logged message SHALL be appended to the session's log buffer as `title: text` followed by a carriage return and line feed. For every level but 5, and while start-up does not hold the file, the log file SHALL be recreated and the whole buffer written into it, so a level-5 line reaches the file with the next message written. The log file SHALL be `<project>.log` in the user folder the settings files live in.

#### Scenario: A level-5 line waits for the next write
- **WHEN** "Windows Error" and "denied" are logged at level 5, then "Script" and "bad role" at level 4
- **THEN** the file is absent after the first and holds "Windows Error: denied\r\nScript: bad role\r\n" after the second

### Requirement: Drawn overlay
The timed overlay SHALL be expired once a frame by the game clock and drawn after every other interface frame while a line waits: the soft rounded box at alpha 0x80 grown by three units' worth on each side, and each line wrapped at the panel's width in the GUI level's main font and the skin's eighth label ink. Nothing SHALL be drawn while the overlay is empty.

#### Scenario: A script report
- **WHEN** a script reports "Script" and "Invalid role set for the witness event." at level 4
- **THEN** "Script : Invalid role set for the witness event." shows in the window's top-left corner and goes after 2 s plus 50 ms a byte

### Requirement: Undrawn status history
Levels 1 to 3 SHALL push the status history (sixteen entries, newest first) without drawing it, as no routine of the original reads it.

#### Scenario: A level-2 message
- **WHEN** a message is logged at level 2
- **THEN** the history's newest entry is its text with kind 1 and nothing new is drawn

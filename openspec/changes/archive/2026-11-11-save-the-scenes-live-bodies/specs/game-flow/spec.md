## ADDED Requirements

### Requirement: End menu pad click
The pad's Select on a showing end menu SHALL play the menus' `MenuClick` at the menu gain with a pitch of 0.8 plus 0.4 times the menu generator's draw, then confirm the selected row; on a hidden end menu it SHALL do nothing.

#### Scenario: Select on the end menu
- **WHEN** the end menu shows and the pad's Select is pressed
- **THEN** one click plays with a pitch from 0.8 to 1.2 and the row is confirmed

### Requirement: Help and GUI scale notifications
The help screen's close SHALL clear the menu's help flag through the flow's help-closed handler. A change of the window's height that moves the GUI scale SHALL lay the menus out again.

#### Scenario: The window grows
- **WHEN** the window's height changes from 720 to 1080
- **THEN** the GUI scale becomes 0.5 and the title menu lays out again

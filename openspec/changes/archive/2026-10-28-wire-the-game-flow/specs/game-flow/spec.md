## ADDED Requirements

### Requirement: Applied flow steps
The running game SHALL apply the flow's menu, end-menu, title-logo, game-view, controls, health, music and quit steps, and a level load's request SHALL hide the menu form and close the end menu while the loaded level stays. The session begin SHALL NOT hide the menu form itself.

#### Scenario: Death and exit
- **WHEN** a reconstructable story player dies, EXIT is confirmed, and 100 ms pass
- **THEN** the end menu shows CONTINUE and EXIT with the first row selected, the exit runs behind the transition, and the title page is shown in menu mode

#### Scenario: Begin
- **WHEN** a session begins around a starter that loads exanima01
- **THEN** the steps are the load, the session task and the end menu's selected flag, with no menu hide

### Requirement: Practice end-menu rows
In practice mode the end menu's first row SHALL leave to the menu form behind the transition on menu 2 and start the next fight on any other menu, and its second row SHALL return to the title behind the transition.

#### Scenario: Practice restart
- **WHEN** RESTART is confirmed on practice's menu 2 at 1000 ms and 100 ms pass
- **THEN** the prelude closes, the transition starts, and the session leaves to the menu form with its theme

### Requirement: Story and arena end-menu rows
In story mode the first row SHALL continue on the reconstructed menu and otherwise exit the story, as the second row does. In arena mode the first row SHALL run the arena's continue and the second return to the title behind the transition. Any other row or mode SHALL do nothing and keep the menu open.

#### Scenario: Story one-row menu
- **WHEN** the story's one-row EXIT menu's row is confirmed
- **THEN** the story exit runs

#### Scenario: Arena third row
- **WHEN** the arena end menu's third row is confirmed
- **THEN** nothing happens and the menu stays open

### Requirement: Launched level session
A level launched directly SHALL run as a practice-mode session without the practice round hook, and a title page the splash put up SHALL stay over it until closed.

#### Scenario: Corpses in a launched level
- **WHEN** `play exanima01` runs 900 frames
- **THEN** no end menu is shown

## ADDED Requirements

### Requirement: Company dialog from NEW CAREER
The arena page's NEW CAREER SHALL close the menu, clear the version line, open the company dialog over the front end with the session state 0x40, move the pointer to the dialog's 370-scaled line and focus the name field. The dialog SHALL restore the emblem the last named NEXT stored when its background word is above 0.

#### Scenario: Opening the dialog
- **WHEN** NEW CAREER is chosen on the arena page
- **THEN** the menu is closed, the company dialog and its emblem form are up, the session state is 0x40 and the name field has the focus

#### Scenario: Reopening after a named NEXT
- **WHEN** NEW CAREER is chosen again after a NEXT stored an emblem with background 7
- **THEN** the emblem form opens on that emblem

### Requirement: Company dialog BACK
The dialog's BACK SHALL run behind the transition, without the music fade: at black the dialog SHALL be freed and the game SHALL return to the title. The dialog SHALL stay up until black and SHALL answer no further button.

#### Scenario: Backing out
- **WHEN** BACK is pressed and the transition runs out
- **THEN** the dialog is gone, the mode is the menu's and the title page is shown

### Requirement: Company dialog NEXT
A named NEXT SHALL store the name and the emblem record, start character creation in the arena's mode and run behind the transition: at black the dialog SHALL be freed and the session state SHALL become the creator's (2). An empty name SHALL keep the dialog and start nothing.

#### Scenario: Naming the company
- **WHEN** NEXT is pressed with the name Ash
- **THEN** the company draft holds Ash and the emblem, the creator is asked once, and after the transition the dialog is gone with the session state 2

## ADDED Requirements

### Requirement: Match board form template
The arena match board SHALL take its tabs, bottom-row buttons, captions,
season picture and item slots from the original's form template, with the
template's captions and initial states.

#### Scenario: Template captions
- **WHEN** the board opens on the standard tab
- **THEN** the tabs read Standard, Specials, Tournament and Season and the
  standard tab shows the "Entry:" and "Prize:" captions

### Requirement: Match board template placement
A template control's x SHALL be snapped to a multiple of four with the
quarter rounded half to even. Its y SHALL be in four-pixel rows, lifted by
half its height less eight when at most 32 pixels tall and by four
otherwise. A bottom-anchored control SHALL stand `y + 8` rows above the
parent's bottom edge, lifted the same way.

#### Scenario: Tabs and bottom row
- **WHEN** the wide board is laid out
- **THEN** the four tabs are 128 by 28 at x 20, 160, 300 and 440 and y 14,
  and End Day, Close, Enter and Forfeit are 128 by 28 at x 20, 160, 300 and
  440 and y 610

#### Scenario: A tall control lifts by four
- **WHEN** a template control on row 60 is 60 pixels tall
- **THEN** its y is 236

### Requirement: Match board day-end and bottom-row actions
The day-end button SHALL be enabled, on the tournament day with any entered
event, once every entered event has a winner, and otherwise once a match
has been played. A click on a shown Close button, an enabled End Day button
or an enabled Forfeit button SHALL report the close, day-end or forfeit
action. Closing SHALL save the tab and three list selections and the next
opening SHALL restore them.

#### Scenario: The tournament's entries decide the day end
- **WHEN** it is the tournament day, one match was played and an entered
  event has no winner
- **THEN** the day-end button is disabled, and it is enabled once that
  event has a winner

### Requirement: Match board show and tournament corrections
On a tournament day with entries, showing the board SHALL switch to the
tournament tab and hide the tab it leaves. A weapon event with nobody
entered SHALL leave the reward panel unmarked. A tournament winner from 2
SHALL read the tournament's company table at entry `winner - 2`.

#### Scenario: The forced switch hides the standard tab
- **WHEN** the board is shown on the standard tab on a tournament day with
  an entered event
- **THEN** the tournament tab is open and the standard tab's fee control
  is hidden

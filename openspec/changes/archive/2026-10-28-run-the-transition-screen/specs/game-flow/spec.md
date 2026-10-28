## ADDED Requirements

### Requirement: Transition screen
Every flow transition SHALL open the transition screen with its action and run the action once the screen passes black: a 600 ms fade, or 2000 ms with a 1000 ms hold when the level exit's fade bit set the long hold before the open; then two frames, a 500 ms hold, a 600 ms fade back in and the close. The screen SHALL run every frame on the real clock whatever covers the game, and an action that changes the level SHALL keep it black until the change is done.

#### Scenario: Action at black
- **WHEN** a revival's transition starts at 1000 ms on the real clock
- **THEN** the alpha is 128 at 1300 ms, 255 with no action at 1600 ms, the revival runs at 1601 ms, the screen stays black to 2130 ms, is at 128 at 2431 ms and closes at 2731 ms

#### Scenario: Long hold from the fade bit
- **WHEN** a level exit with the fade bit opens the screen at 0 ms
- **THEN** the alpha is 128 at 1000 ms, nothing loads at 2000 ms and the level loads at 2001 ms

#### Scenario: Held during the load
- **WHEN** a named-locale exit reaches black at 601 ms and its level loads until 5000 ms
- **THEN** the screen stays at 255 through the load and the 500 ms hold after it, and is at 128 300 ms into the fade in

### Requirement: Transition music fade
With the caller's music fade on, the main track's gain SHALL fall as (1 - t) times its gain, stop at black with the audio fade at 0, and the audio fade SHALL rise with the fade in to 1 at the close; the world fade SHALL reset to 1 at black. The quits, the respawn, RESTART and the next fight into the last fight SHALL fade it; the revival and the class cancel SHALL NOT; a level exit SHALL fade it when its destination's music differs.

#### Scenario: Music fade
- **WHEN** the story CONTINUE after death opens the screen with the music at 0.5
- **THEN** the music is 0.25 at 300 ms, stops at black with the audio fade at 0, and the audio fade is 0.5 halfway through the fade in and 1 at the close

#### Scenario: Next fight into the boss
- **WHEN** NEXT FIGHT is confirmed after fight 6 and after fight 7
- **THEN** only the second fades the music

### Requirement: Session frame phase
The session frame task SHALL run the dialogue and the hook while no main menu covers the game in every phase but the transition's (phase word 0).

#### Scenario: Ended session
- **WHEN** an ended session with no hook runs a frame
- **THEN** only the dialogue ticks

## MODIFIED Requirements

### Requirement: Applied flow steps
The running game SHALL apply the flow's menu, end-menu, title-logo, game-view, controls, health, music and quit steps, and a level load's request SHALL hide the menu form and close the end menu while the loaded level stays. The session begin SHALL NOT hide the menu form itself.

#### Scenario: Death and exit
- **WHEN** a reconstructable story player dies, EXIT is confirmed, and the transition screen reaches black
- **THEN** the end menu shows CONTINUE and EXIT with the first row selected, the exit runs behind the transition, and the title page is shown in menu mode

#### Scenario: Begin
- **WHEN** a session begins around a starter that loads exanima01
- **THEN** the steps are the load, the session task and the end menu's selected flag, with no menu hide

### Requirement: Practice end-menu rows
In practice mode the end menu's first row SHALL leave to the menu form behind the transition on menu 2 and start the next fight on any other menu, and its second row SHALL return to the title behind the transition.

#### Scenario: Practice restart
- **WHEN** RESTART is confirmed on practice's menu 2 and the transition screen reaches black
- **THEN** the prelude closes, the transition starts, and the session leaves to the menu form with its theme

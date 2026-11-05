# Spec Delta

## MODIFIED Requirements

### Requirement: Mode switch
A switch to another game mode SHALL clear the reconstruction state, store the outgoing mode's player, and set the hook: the story's for story, none for the arena, the title and any other word. A registered scripted mode's enter hook SHALL run; an unregistered word SHALL only become the mode word. Practice SHALL keep its hook, and entering it from outside the practice family SHALL restart the round clock.

#### Scenario: Into practice from the title
- **WHEN** the title switches to practice at 1000 ms
- **THEN** no hook is installed and the round clock reads 1000

#### Scenario: Into the versus lobby's word
- **WHEN** the flow switches to `0x1010` with no scripted mode registered
- **THEN** the hook is removed, no scripted hook runs and the mode word reads `0x1010`

### Requirement: Session bookkeeping
Leaving a session SHALL stop the hook and frame task, close the game view, the dialogue and the prelude, and run the leave hook of the arena or of a registered scripted mode. It SHALL then close the save package and hide the end menu before the world goes. Back to the title SHALL show the main menu's title logo and leave the GUI scale alone. A session's begin and continue SHALL clear the end menu's no-row-selected flag. A story continue SHALL clear only the first wake-up's animating flag.

#### Scenario: Back to the title
- **WHEN** a story returns to the title
- **THEN** the title logo shows and neither the help flag nor the GUI scale is written

#### Scenario: A registered word without the scripted bit
- **WHEN** a session of a registered word without the `0x1000` bit begins and then leaves
- **THEN** its begin hook runs and its leave hook does not

## ADDED Requirements

### Requirement: Scripted-mode table
The flow SHALL hold a table of scripted game modes, registered by word, each with the enter, begin, leave, creator-cancel and end-menu hooks. Every lookup SHALL take the first entry with the word. A session's begin SHALL run the mode's begin hook before the frame task starts. The session leave SHALL run the mode's leave hook only for a word with the scripted bit (`0x1000`). With nothing registered, as in Exanima 0.9.5, no scripted hook SHALL run anywhere.

#### Scenario: A word registered twice
- **WHEN** a word is registered twice and its end menu row is clicked
- **THEN** the first entry's handler receives the row

### Requirement: Creator and class-select cancels
The creator's cancel, outside the arena recruit, SHALL run behind the transition with the music left playing. At black it SHALL run the mode's cancel hook when the mode is registered with one, looked up by word alone, and then go back to the title. The class select's cancel SHALL go back to the title behind the transition and SHALL run no scripted hook.

#### Scenario: Creator cancel in a registered mode
- **WHEN** the creator is cancelled in a registered `0x1010` with a cancel hook
- **THEN** a transition starts, and at black the cancel hook runs before the leave hook and the title shows

#### Scenario: Creator cancel with nothing registered
- **WHEN** the versus creator is cancelled with nothing registered
- **THEN** no scripted hook runs and the title shows behind the transition

### Requirement: Versus lobby gate
The arena page's VERSUS row SHALL appear only when the versus flag is set. Exanima 0.9.5 never sets it, and Exarust sets it only with the `--versus` debug option. VERSUS SHALL switch the mode word to `0x1010` before the free creator opens with READY.

#### Scenario: Retail arena page
- **WHEN** the arena page is shown without the versus flag
- **THEN** it reads NEW CAREER, CONTINUE, PRACTICE and BACK

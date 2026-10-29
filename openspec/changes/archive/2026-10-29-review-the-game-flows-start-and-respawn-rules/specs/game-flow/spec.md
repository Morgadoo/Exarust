## ADDED Requirements

### Requirement: Story start hand-over
A new story SHALL hand the loader the level-start point (8200, 25, 3920), the view forward along +z looking at (8200, 0, 4000) and a start record of the background's start motion followed by two zero words; a practice start's view SHALL be forward along −z looking at (0, 20, 500).

#### Scenario: A knight's start record
- **WHEN** a knight's story starts
- **THEN** its start record is the waking motion, 0, 0 and its spawn point is (8200, 25, 3920)

### Requirement: First wake-up end
While the first wake-up animation runs, the story hook SHALL end it once the player's animator word no longer holds 0x20, whatever the condition word holds.

#### Scenario: A waking condition with a clear animator word
- **WHEN** the condition holds 0x20 and the animator word does not
- **THEN** the first wake-up animation ends

### Requirement: Respawn and revive closes
A revival SHALL close the prelude before its save; the story respawn SHALL hide the end menu and close the prelude after its save and before the world goes, and SHALL NOT take the player's controls away.

#### Scenario: The story respawn
- **WHEN** the death screen's respawn runs
- **THEN** the end menu hides and the prelude closes, and no controls-off step is made

### Requirement: Death and continue details
The death path SHALL show the alive tip when the health is at least 0.01 in double precision or unordered; the death screen's CONTINUE SHALL write the player back with record flags 0x100; the next practice fight SHALL clear the player's condition word.

#### Scenario: A NaN health
- **WHEN** a story player dies with a NaN health
- **THEN** tip 2 shows and the save is neither dropped nor the player stripped

### Requirement: Sixth-level stamp
A level exit that makes the sixth level's checkpoint SHALL also stamp the level time; other checkpoints SHALL NOT.

#### Scenario: The third level's checkpoint
- **WHEN** a level-3 exit makes its checkpoint
- **THEN** no level-time stamp is made

### Requirement: Launch unlock words
The boot SHALL read the expert unlock from settings path [1, 0x6431ac01] and the background unlock from [1, 0x6431ac02], leaving a word unchanged when its entry is missing.

#### Scenario: A missing entry
- **WHEN** the store has neither entry and the expert ladder is already unlocked
- **THEN** it stays unlocked

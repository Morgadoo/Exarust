## ADDED Requirements

### Requirement: Queued weapon switch
A swap request on a body without a busy bit (0x10202) and no drag SHALL swap the sets at once when the character's own inventory window is open, and otherwise SHALL write the switch motion into the body's requested motion, which the body tick starts when no busy bit (0x10203) is set.

#### Scenario: Pack closed
- **WHEN** the player presses the swap key standing, with the inventory closed
- **THEN** the switch motion is requested and the sets stay

#### Scenario: Pack open
- **WHEN** the player presses the swap key with its own inventory open
- **THEN** the sets swap at once and no motion is requested

### Requirement: Switch point swap
While a body plays the switch clip, the posture SHALL write the switching word up to 60 % of the clip (compared in double precision; a NaN progress stays switching) and the switched word past it; when it writes the switched word from a condition without bit 4 it SHALL post the body's weapon-set swap, which the frame makes after the step, for any body that keeps possessions.

#### Scenario: Once a switch
- **WHEN** a body's switch clip is at 59 %, then 61 %, then 70 %
- **THEN** the sets stay, then swap, then stay swapped

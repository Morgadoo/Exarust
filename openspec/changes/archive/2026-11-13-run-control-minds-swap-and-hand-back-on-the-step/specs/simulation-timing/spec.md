## ADDED Requirements

### Requirement: Control Mind's swap runs in the step
Control Mind's handover SHALL hold the caster's player behaviour and set aside the target's AI on the world that walks the powers, together with its status bits, the possession marker, the new player behaviour and the player marker. Only the GUI sweep, the rooms' anchor and the held body the frame keeps SHALL be sent to the frame.

#### Scenario: A handover in free mode
- **WHEN** the handover runs while the simulation runs free and several steps reach the frame in one frame
- **THEN** after the handover the frame shows the player behaviour on the target only and no duellist on the target

### Requirement: Control Mind's hand-back is posted to the step
At the transition's black the frame SHALL close the GUI's forms, move the rooms' anchor back and forget the held body. It SHALL post the hand-back to the start of the next step: the status words, the target's AI and the caster's player behaviour back, and the player marker on the caster's root. The frame SHALL NOT write the held duellist or player behaviour.

#### Scenario: Control Mind ends in free mode
- **WHEN** Control Mind ends while the simulation runs free and the step changes the duellist and the player behaviour every step
- **THEN** the duellist's roll and the caster's aim never go back on the frame, and the log counts no Duellist or PlayerBody among the frame values applied after a newer simulation write

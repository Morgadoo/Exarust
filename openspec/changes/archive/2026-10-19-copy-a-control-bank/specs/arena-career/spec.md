## ADDED Requirements

### Requirement: Control bank
When a focus clears a session mark that is not already 0, a flag SHALL become 0 and a code SHALL become 4. Side 0 SHALL copy sixteen words from the first bank when its flag is not 0, and then clear that flag. Any other side SHALL raise a notice and copy the second bank when the flag is 0, and then set the flag. A refused copy SHALL leave the words. An install after the copy SHALL be counted. The install body is not part of this step.

#### Scenario: A cleared mark copies one bank
- **WHEN** a match begin follows a focus whose mark is 1, on side 0 with a set flag, and another begin uses side 1 with a clear flag, and a third finds the mark already 0
- **THEN** the first copies the first bank and sets the code to 4, the second copies the second bank and raises the notice, and the third leaves the code and the words

#### Scenario: A matching flag refuses the copy
- **WHEN** side 0 finds the flag already 0, and a begin with no focus uses mask 1
- **THEN** the words stay, the focus still counts an install, and the mask-only begin copies nothing

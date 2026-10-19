# Spec Delta

## ADDED Requirements

### Requirement: A dead entry sets the race hands and the step
When both pair words are 0, class 0x800 and body kind 0x100 SHALL set the hands to firm and open, leave the bare mark on, clear the class word, and set the primary to -1. Class 0x500 SHALL mark the hands and clear the bare mark. A pair word of 1 SHALL leave the hands. Feral 0 and feral -1 with a base of 1 SHALL score reach bits 0x42a00000, and feral 1 SHALL score bits 0x42600000. Empty slots SHALL fill. An offset of 3, 4, 0 SHALL face along x. A walking body SHALL skip this.

#### Scenario: Firm and open hands, then a marked class
- **WHEN** a dead body with both pair words 0, class 0x800 and body kind 0x100 is entered, then class 0x500 at feral 1, then a pair word of 1 at feral -1, and a walking body is entered
- **THEN** the hands are firm and open with the bare mark on, the class word is 0 and the primary is -1, the reach bits are 0x42a00000, the slots are filled and the facing is along x, class 0x500 is marked with reach bits 0x42600000, the pair word of 1 leaves the hands and scores reach bits 0x42a00000, and the walking body is unchanged

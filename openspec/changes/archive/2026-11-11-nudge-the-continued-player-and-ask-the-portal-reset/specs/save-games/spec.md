## ADDED Requirements

### Requirement: Reset to portal question
After each select of the saves' character screen that reads a slot, and when the screen opens, the story SHALL raise the `Reset to portal` box when the save read has the third byte of its player data flags at 0 and holds an `exanima06` section, or holds `exanima04` with a world object database magic other than 0xDBAC0D06. The arena SHALL never raise it.

#### Scenario: A pre-0.9.5 save past the portal
- **WHEN** the screen opens over a story save with that byte cleared and an `exanima06` section
- **THEN** the box asks "Do you wish to reset all progress after the portal?" with Yes and No

### Requirement: Reset to portal answer
The box's Yes SHALL delete the slot file the select read and, unless the slot's checkpoint still opens, drop the slot from the list and clear the current slot; the same index SHALL then be selected again. No and the cancel key SHALL close the box and change nothing.

#### Scenario: Yes without a checkpoint
- **WHEN** Yes is chosen for a slot with only its save file
- **THEN** the save file is gone and the list is empty

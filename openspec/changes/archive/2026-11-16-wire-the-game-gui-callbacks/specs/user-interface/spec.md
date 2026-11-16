## ADDED Requirements

### Requirement: Inventory page buttons swap the sets
The inventory's Primary and Secondary page buttons SHALL share one record callback: the button a press turns on SHALL ask the character the window shows to swap its weapon sets as the swap key does, refused while the body's state has any of the bits 0x10202 or an item drag is in flight, and made at once because the window is open; the button it turns off SHALL do nothing.

#### Scenario: Secondary pressed
- **WHEN** the player's inventory is open with Primary lit and the player presses and releases Secondary
- **THEN** the player's weapon sets swap, and a press on the lit Primary asks for nothing

### Requirement: Journal entry list drops down
The journal's selection SHALL keep the entry list's 1-based row, the entry count and every title; a release over the list SHALL open its drop-down list of those titles under it with the row chosen, and a row picked there SHALL store the text box's text into the selected entry and select the picked entry, whose title and text the list and the box then show.

#### Scenario: Picking the first entry
- **WHEN** the journal has two entries, the second selected with text typed, and the player opens the list and picks its first row
- **THEN** the second entry keeps the typed text and the first entry's text shows

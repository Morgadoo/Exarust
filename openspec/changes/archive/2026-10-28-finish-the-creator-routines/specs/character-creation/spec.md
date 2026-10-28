## MODIFIED Requirements

### Requirement: Creation entry
Opening the screen SHALL plan its record by screen mode. Recruiting SHALL make a new default record without the gear filter. Story SHALL load the chosen background, clear the record's experience and filter it. Other modes SHALL keep the record they have or run the free creator. Every record but a continued one SHALL be set to full health by four `0xffff` halves, and every filtered record SHALL then get the underwear pass.

#### Scenario: Modes
- **WHEN** recruiting
- **THEN** a new record is made, marked, not filtered and set to full health
- **AND** its yellow, red, focus and save seed words are `0xffff`
- **WHEN** the free story creator finds no preset
- **THEN** a default record is made, marked and not filtered

## ADDED Requirements

### Requirement: Creation new-character mark
A record the screen makes SHALL be marked new: always when recruiting, when the screen had none in the free creator and in continue, and in story only for a fresh record that reads a saved preset or starts the unknown background. A named background without a preset adopts its template unmarked.

#### Scenario: Marks
- **WHEN** story opens a named background with no preset and no record
- **THEN** the template is adopted and not marked
- **WHEN** story reads a saved preset with no record
- **THEN** the fresh record is marked

### Requirement: Creation back switch
The equipment page's tab SHALL hide only the first built of the weapons panel, the skills panel and the versus lobby form, sliding the skills panel by `0x2d` only when it is the one hidden, then hide the versus skills panel and the versus equipment form when they exist. A non-continued character SHALL show its two appearance panels again and set both tab words to `0x110`. From the appearance page the tab SHALL leave the creator: to the arena for a recruit, otherwise to the menu.

#### Scenario: Back
- **WHEN** weapons and skills are both built
- **THEN** only the weapons panel is hidden and nothing slides
- **WHEN** only the skills panel is built
- **THEN** it is hidden and slides by `0x2d`
- **WHEN** a recruit leaves from the appearance page
- **THEN** the arena's form opens

### Requirement: Creation preview routines
The preview material SHALL be applied, when the generator exists, to items 0 through the list's highest index. The icon sheet's icon size SHALL be eight GUI units and the dummy-head entry height two more. The continue view SHALL refresh the first listed item on each of its passes only when that item's record carries flag `0x100`, and the extra item on each pass when it exists. The sex radio SHALL act only when checked and SHALL open the audio device only when it was not ready.

#### Scenario: Loops
- **WHEN** the item list's highest index is -1
- **THEN** no item gets the material
- **WHEN** the highest index is 3
- **THEN** four items get it
- **WHEN** the GUI unit is 5
- **THEN** the icon size is 40

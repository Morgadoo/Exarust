## MODIFIED Requirements

### Requirement: Creation kit match
An apparel entry SHALL be offered unless a worn item matches it by id, or by base id and the entry's four colour-layer bytes compared in layer order; a record without a colour-layer block SHALL NOT match by base. A weapon entry SHALL be offered unless one of the first two hands has its base id.

#### Scenario: Held items
- **WHEN** a worn item's base id and its four layer bytes match an entry
- **THEN** that entry is not offered
- **WHEN** only the fourth layer byte differs
- **THEN** the entry is offered

### Requirement: Creation kit item
A plain kit entry SHALL add the item itself. An item with an appearance block whose alias marker is `@` SHALL add the alias target with the entry's variant and three tint bytes; any other marker SHALL add the entry unchanged. Any other item SHALL become a new instance with the entry's variant, its four colour-layer bytes (each layer marked with five) and the second and third tint bytes as dyes where non-zero; an instance that cannot be made SHALL add nothing.

#### Scenario: Plain, instance and alias
- **WHEN** a plain entry 12 resolves
- **THEN** item 12 is added with no appearance
- **WHEN** an instance entry with tint 2, 0, 4 and layers 5, 6, 7, 8 gets new item 30
- **THEN** item 30 takes layers 5, 6, 7, 8 and dye bytes unchanged, 4
- **WHEN** an alias entry with marker `@` resolves to item 21
- **THEN** item 21 takes the variant and the three tint bytes

## ADDED Requirements

### Requirement: Creation equipment panels
The apparel panel SHALL be bound as the current equipment form as soon as it is made, before the mode's table is picked, and an existing panel SHALL be shown and bound without consulting the table; only a made panel with a table SHALL be filled and finished. The skills page SHALL hide the second preview panel only when it exists, flag the skills panel with 0x20000 only when it makes it, and rebind an existing panel to the edited record.

#### Scenario: A named story background
- **WHEN** the apparel panel is made for a named story background
- **THEN** it is the current equipment form with no items and no finish

#### Scenario: The skills page again
- **WHEN** the skills page opens with the panel already made
- **THEN** the panel gains no flag and its record is rebound

### Requirement: Creation continue page texts
Opening the continue page SHALL, when the save manager's list has entries, select the first of the list's count in the label, show the first entry's display string and put the first entry's name in the name box, keeping row 0; an empty list SHALL clear both texts and leave the kept row.

#### Scenario: Three saves
- **WHEN** the page opens on three entries
- **THEN** the label selects 1 of 3 with the first display string and the name box shows the first name

# Spec Delta

## ADDED Requirements

### Requirement: Figure drag-over by race
Before its drop, the character view SHALL test the dragged item against the figure's race. A weapon or shield whose section flags carry `0x2000000` SHALL be taken only by a race whose kind word has `0x10000`. Any other weapon with `0x4000000` SHALL be taken only by a race whose kind word has `0x20000`. A refused drag SHALL end as a cancelled one, and the item SHALL go back to its source.

#### Scenario: A golem's sword on the player
- **WHEN** the golem sword (item 0x42, flags 0x13000011) is dropped from the pack onto the player's figure
- **THEN** the human's hands stay empty and the drag is cancelled

#### Scenario: A golem takes it
- **WHEN** the same sword is dropped on a figure of the Golem Guard race
- **THEN** the drop goes on and the sword is taken into the hands

### Requirement: Apparel on the figure by race
The view SHALL take apparel only when it allows changes and the figure's race look word has bit 4. The drop SHALL read the same bit. An item with no apparel, weapon or shield section SHALL pass only as the AL crystal (500), on a race whose flag word has `0x2000`, while the view shows no item.

#### Scenario: A golem refuses a tunic
- **WHEN** a tunic is dropped on a figure of a golem race (look word without bit 4)
- **THEN** the drag is cancelled

### Requirement: Trade preview race
The trade window's preview SHALL run the same drag-over test. It SHALL read the look word of the race that the record's race byte names (race id = byte + 1).

#### Scenario: Human preview
- **WHEN** a golem sword from the stock is dropped on a human preview
- **THEN** the drag is cancelled and the preview's hands are unchanged

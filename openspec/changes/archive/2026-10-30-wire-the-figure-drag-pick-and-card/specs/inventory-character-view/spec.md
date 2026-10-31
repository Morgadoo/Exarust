# Spec Delta

## ADDED Requirements

### Requirement: Figure pick
A pick on the figure SHALL cast the screen point's ray from the view's camera and SHALL take the nearest hit. It first tests the faces of the dressed figure's merged mesh: a face is hit only from the side it turns toward the ray, and it answers the worn item it belongs to (none for the body's skin). Then each other drawn mesh is tested, and counts only when nearer. A held item's meshes answer that hand's item and slot; any other mesh (the hair) answers no item.

#### Scenario: Pointer on the helmet
- **WHEN** the player wears a kettle helm and the pointer is over the helmet on the figure
- **THEN** the pick answers the helm's id and no slot

#### Scenario: Pointer on the blade
- **WHEN** the player holds a falchion in the main hand and the pointer is over the blade in front of the leg
- **THEN** the pick answers the falchion's id and slot 1

### Requirement: Figure pick flags
The picked item's flags SHALL come from the hand's record entry for a held item, else from the first worn entry naming the item, else 0.

#### Scenario: Worn helm flags
- **WHEN** the pick answers a worn helm whose worn entry has flags 0x40
- **THEN** the picked flags are 0x40

### Requirement: Drag off the figure
A left press on the figure SHALL start a drag of the picked item with the figure drag start's payload. A release on a pack panel SHALL run that panel's accept test and drop, then take the item off the figure. A release anywhere else SHALL cancel and leave everything in place.

#### Scenario: Release outside the panels
- **WHEN** a drag off the figure is released over the world view
- **THEN** the item stays on the figure and no panel changes

### Requirement: Held item taken off
A held item dragged off the figure SHALL leave its hand by the scheduled take-out: the off hand's weapon or light moves into an emptied main hand. When the item went to a panel other than the player's pack, the player's pack SHALL drop its held entry of the item.

#### Scenario: Weapon into the pack
- **WHEN** the held falchion is dragged off the figure into the player's pack
- **THEN** both hands are empty and the pack's held entry of the falchion is shown again, unmarked, at the drop point

### Requirement: Worn item taken off
A worn item dragged off the figure SHALL leave the worn list, then the underwear rule SHALL run, the outfit SHALL lose the item's garment, and the player's pack SHALL be charged the item's bulk once.

#### Scenario: Helmet into the pack
- **WHEN** the worn helm is dragged off the figure into the player's pack
- **THEN** the helm is in the pack at the drop point and off the worn list, and the braies are back on

### Requirement: Card from the figure
A right press on the figure SHALL open, at once and at the pointer, the card of the item the pick finds, and nothing when it finds none.

#### Scenario: Card of the worn helm
- **WHEN** the right button is pressed over the helmet on the figure
- **THEN** the helm's card opens at the pointer

# inventory-character-view Specification

## Purpose
The inventory's character view: the player's figure drawn in the doll panel, the encumbrance dots beside the Encumbrance label, and the drop that dresses the player.

## Requirements

### Requirement: Figure in the character view
While the inventory is open the system SHALL draw the player's dressed figure in the character view (the doll panel less its 2-pixel border) from a camera 410 cm from the figure's point 100 cm above its feet, towards (−200, 60, 360), with a 30° field of view, lit from (0.5, 0.7, 0.5) in the camera's space, and SHALL release the camera, its image and the figure when the inventory closes.

#### Scenario: The view at unit 4
- **WHEN** the inventory opens at (8, 8) at GUI unit 4
- **THEN** the figure is drawn in the 224 × 380 view at (18, 18), the canvas is cleared there and the camera clears to the canvas's colour there (black at alpha 227)

### Requirement: Encumbrance dots
The inventory's rating-dots record SHALL show ten times the player's encumbrance: whole dots up to the value less a quarter, then a half dot, three units apart (two and a half above 7.25), 22 pixels square, their top the record's top plus half of its height less 22 shifted as an unsigned 64-bit value, in the label group's seventh ink.

#### Scenario: Half encumbered
- **WHEN** the player's encumbrance is 0.25 at unit 4
- **THEN** two whole dots and a half dot draw 12 pixels apart from x 100 of the client, 3 pixels below the record's top at client y 386

### Requirement: Drop on the figure
A drop from a pack or container onto the figure SHALL wear apparel through the equip conflict pass (a locked piece in the way refuses it; displaced pieces return to the pack, braies are dropped), SHALL take a weapon into the hands by the take-in plan, and SHALL take a shield into the off hand; the inventory's pack SHALL keep the item's bulk because the view is its linked panel.

#### Scenario: A second sword
- **WHEN** a one-handed sword is dropped on a figure already holding one
- **THEN** the new sword is in the main hand and the old one's carried copy loses its held mark

### Requirement: Insight's off-hand drop
A weapon dropped on the right half of the character view (the view's width halved less than the pointer's view x) by a character who knows Insight's technique 8 among the first points / 5000 techniques SHALL be taken into the off hand.

#### Scenario: One-hander on the right half
- **WHEN** a one-handed sword is dropped on the right half by a player with 5000 Insight points and technique 8, the main hand holding item 8
- **THEN** the sword goes into the off hand beside item 8

### Requirement: Figure drag start
A drag started on the figure SHALL, with the mind cost charged, store the item's cost (twice a weapon's desirability, else an apparel's protection, else 0) and refund it, SHALL cancel when the character cannot take the charge, and SHALL otherwise answer 0x105 with kind 0xFB01 (all ones for a locked view's item flagged 0x80000000) and a payload of the icon, the item's flags and half the icon size; an icon that does not resolve SHALL start no drag while the charge stands.

#### Scenario: A weapon of desirability 1.25
- **WHEN** a weapon of desirability 1.25 with flags 0x10 and icon 4 is dragged at icon size 160 with the mind cost on
- **THEN** the cost 2.5 is stored and refunded and the payload carries kind 0xFB01, icon 4, flags 0x10 and half 80

### Requirement: Figure hand scene
While the character view is open, each figure hand whose record item differs from the one its model shows SHALL get that item's model, when it has one, as a weapon (the hand's held-slot grip, flipped by `0x1000000`), a shield (left forearm frame at the hand's rest offset, −5° about z) or a light (its grip for that hand): old models released, new ones attached, the figure's held items set (its moveset follows), the stance restarted with blend 20. A hand left empty SHALL release its model. 
#### Scenario: Weapon and shield
- **WHEN** a one-handed axe and then a buckler are dropped on the figure
- **THEN** the axe hangs in the right hand by its grip and the buckler on the left forearm, and the figure's off hand shows a shield

#### Scenario: Two weapons
- **WHEN** a one-handed axe is dropped, then a second one on the view's right half by a player who knows Insight's technique 8
- **THEN** both hands show an axe, the second by the off hand's grip

### Requirement: Carried torch on the figure
A carried torch outside the record SHALL show on the figure in the hand the player holds it in when the record leaves that hand empty, else in the other hand when that one is empty.

#### Scenario: Weapon beside the torch
- **WHEN** a player carrying the torch in the right hand takes a one-handed axe
- **THEN** the axe replaces the torch in the right hand and the torch shows in the left, raised by the one-hander-and-torch moveset

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

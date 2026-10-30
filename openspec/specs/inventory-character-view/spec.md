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

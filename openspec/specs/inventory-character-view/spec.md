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

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

### Requirement: Mind cost of a drag off a bound body
With the inventory's mind window set (the shown body bound by a Loyalty or possession marker), a press that starts a drag off the figure SHALL lower the body's trust by the item's cost (twice a weapon's desirability, else an apparel's protection value, 0 for anything else) and keep that cost.

#### Scenario: Drag a weapon off a bound body
- **WHEN** the player drags a weapon off a bound body's figure
- **THEN** the body's markers lose twice the weapon's desirability from their trust

### Requirement: Binding ended by the view
Each marker that takes a trust change SHALL run the Loyalty marker's limit test; the first that fails SHALL end, the inventory SHALL close and the drag or drop SHALL do nothing. The test SHALL run even for a change of 0.

#### Scenario: Hostile bound body
- **WHEN** a press on a hostile bound body's figure runs the test and its marker fails
- **THEN** the marker ends and the inventory closes

### Requirement: Cancelled drag gives the cost back
A drag off a bound body's figure that is cancelled, or dropped back on the view, SHALL raise the body's trust by the cost its start kept.

#### Scenario: Release outside every panel
- **WHEN** the drag ends outside every panel
- **THEN** the body's trust is back where it was

### Requirement: Mind cost of a drop on a bound body
With the mind window set, a drop of apparel SHALL raise the shown body's trust by the piece's wear room, and a drop of a weapon by twice its desirability less what its take-in displaces from the hands, before the item is taken.

#### Scenario: Weapon into an empty hand
- **WHEN** a one-handed weapon of desirability 2.4 is dropped on a bound body with empty hands
- **THEN** its markers' trust rises by 4.8

### Requirement: Displaced held shield weight
A held shield displaced by a take-in SHALL weigh the low half of its desirability's double read as a single, as the original's caller reads that answer.

#### Scenario: Shield of an exact value
- **WHEN** a held shield whose value is exactly 2 is displaced
- **THEN** it weighs 0

### Requirement: HasItem searches carried containers
A script's HasItem SHALL answer yes for any item in the character's carried list and for anything inside a carried container, nested containers included, read from the containers' records.

#### Scenario: Key in a bag
- **WHEN** a script asks whether a character carrying a bag holds the key inside it
- **THEN** it answers yes

### Requirement: Shield drop by the shield take
A shield dropped on the figure SHALL be taken by the shield take: the off hand's item taken out first and the record's off hand given the shield with all its flags.

#### Scenario: Shield over a dagger
- **WHEN** a shield is dropped on a figure holding a dagger in the off hand
- **THEN** the dagger leaves the hand and the shield takes it

### Requirement: Swap gate inputs
The weapon-set swap's gate SHALL refuse the swap while an item drag of the GUI is in flight, and SHALL answer an immediate swap for a body whose inventory is open.

#### Scenario: Swap key during a drag
- **WHEN** the swap key is pressed while an item is dragged
- **THEN** no swap is asked for

### Requirement: Page buttons follow the weapon set
The open inventory SHALL light its Primary or Secondary page button by the set its body has out, and a swap while it is open SHALL light the other one and load the pack again from the record.

#### Scenario: Swap with the inventory open
- **WHEN** the shown body swaps from its second set to its first
- **THEN** Primary is lit and the pack shows the newly held items as held

### Requirement: Arsenal keys and right release
The open arsenal SHALL take each key of the frame with its word (a key-down or its repeat: Escape closes, the bound inventory key flips the embedded inventory; a typed character does nothing; nothing goes on to its parent) and a right release over its form, which closes it.

#### Scenario: Right release over the arsenal
- **WHEN** the player releases the right button over the open arsenal
- **THEN** the arsenal closes and the player's controls come back

#### Scenario: A held inventory key
- **WHEN** the inventory key goes down and repeats on the open arsenal
- **THEN** the embedded inventory flips on the key-down and again on the repeat

### Requirement: Open pack drains its crystals
While the player's own inventory is open, the inventory crystal-drain source SHALL drain the crystal stacks of the inventory's pack panel instead of the record, resolve each drained entry's icon again for its new flags, and give the carried entry it was loaded from the same flags.

#### Scenario: A crystal in the open pack
- **WHEN** a game second passes with an AR crystal stack of charge 10 in the open pack
- **THEN** the pack entry and the carried entry it came from both carry charge 8

### Requirement: View shows the worn core crystal
When the inventory's character view is made for a body, it SHALL show the first of the body's worn items that is one of the crystals A, AL, B, C and AR, and show nothing for a body that wears none; the shown item SHALL go when the view closes.

#### Scenario: A golem wearing its core
- **WHEN** the inventory opens on a body that wears crystal A
- **THEN** the view shows crystal A with its flags

### Requirement: Shown item draw and hover
The view SHALL draw the shown item's icon at its lower-left corner (the icon placed at one unit from the left and one unit above the bottom by the icon draw's origin rule), clipped to the view, and its hover pick SHALL answer the shown item first when the pointer is on an opaque pixel of that icon.

#### Scenario: The pointer on the shown icon
- **WHEN** the pointer presses on the shown crystal's icon
- **THEN** the pick answers the crystal and a drag of it starts

### Requirement: Core crystal drop and take-off
A drop of an item the body takes as its core SHALL be accepted only while the view shows nothing; it SHALL leave its source and be shown, without entering the record's lists. A release of a drag of the shown item SHALL take it off the worn list and show nothing.

#### Scenario: The AL crystal dropped on a golem's figure
- **WHEN** an AL crystal is dragged from the pack onto the figure of a body whose race takes it
- **THEN** it leaves the pack and is shown, and a second one is refused

### Requirement: View image supersampling
The inventory's character view SHALL render its image at the supersampling level its constructor takes from the settings: two image pixels a view pixel on each axis with super sampling on (4x), one with it off.

#### Scenario: Super sampling off
- **WHEN** the inventory opens with super sampling off
- **THEN** the view's image is the view's own size

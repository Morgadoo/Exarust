# carried-items Specification

## Purpose
What a character wears, carries and keeps in a container: equipping by occupancy, the underwear rule, and moving an item between a container and a pack.

## Requirements

### Requirement: Equip by occupancy
Putting an item on SHALL take off every worn item whose occupancy shares a sub-part on the same layer of any bone. Displaced braies SHALL be dropped. Any other displaced item SHALL return to the pack. A human left with nothing over the groin SHALL wear braies again.

#### Scenario: Trousers over braies
- **WHEN** a human wearing braies puts on trousers that cover the groin, then takes the trousers off
- **THEN** the braies are not in the pack while the trousers are worn, and they are worn again once the trousers are off

#### Scenario: Another layer
- **WHEN** a shirt on a higher layer is worn under a coat that occupies that same layer
- **THEN** the shirt returns to the pack and the coat is worn with the braies

### Requirement: Container transfer
Moving an item SHALL take it out of the source list and append it to the destination. Moving an id the source does not hold SHALL leave both lists unchanged.

#### Scenario: Out of a chest
- **WHEN** a chest holds an item and a pack does not, and the item is moved to the pack
- **THEN** the chest no longer holds it and the pack does, and moving it again does nothing

### Requirement: Weapon set swap
A character's possessions SHALL hold an out and a stowed weapon set of a main and an off hand, and a carried list in which an item in a hand stays listed with its held bit 0x100. Swapping the sets SHALL toggle the record's second-set flag 0x0100_0000, unmark each going item's held entries once per hand it fills less the incoming hands that hold it, and mark the incoming items held the same way, a two-handed item counting once. The sets SHALL trade places only when a hand's item changes.

#### Scenario: A sword for a two-handed axe
- **WHEN** a sword is out and a two-handed axe fills both stowed hands, and the sets are swapped and swapped back
- **THEN** after the first swap the sword's entry is unmarked, both axe entries are held and the flag is set; after the second the marks and the flag are as before

#### Scenario: An item in both sets
- **WHEN** the same item is in the out main hand and the stowed main hand
- **THEN** a swap keeps its held mark and the main hand does not change

#### Scenario: The same items in both sets
- **WHEN** both sets hold the same sword in the main hand, the stowed one marked lit, and the sets are swapped
- **THEN** the flag is set, no hand plan is made and the out sword stays unlit

### Requirement: Hands after a swap
The hands SHALL take what changed: a weapon as a weapon, a light weapon in the off hand as a light, a shield as a shield, another item with a light section as a light, anything else as a weapon. Only a weapon SHALL take the stored lit and flipped bits.

#### Scenario: A torch in the off hand
- **WHEN** the incoming set holds a flipped weapon in the main hand and a lit torch in the off hand
- **THEN** the main hand takes a flipped weapon and the off hand a burning light

### Requirement: Giving and having
Giving an item to a character with no open pack SHALL append it at the pack's origin; with the character's view open its pack panel SHALL take it at its centre and place it, and an item the panel refuses SHALL be lost. Giving back an item taken out of a hand SHALL list it at 40 by 4 sixteenths of a unit. Asking whether a character has an item SHALL search its list and every container in it, at any depth.

#### Scenario: An item in a pouch in a bag
- **WHEN** a character carries a bag holding a pouch holding an item
- **THEN** it has the bag, the pouch and the item, and not an item none of them holds

#### Scenario: A refused gift
- **WHEN** an item without an icon is given while the character's view is open
- **THEN** the panel refuses it and the carried list does not gain it

### Requirement: Taking one-handed items
A one-handed item SHALL go to a free main hand. With the main hand full, a light beside one weapon or the fists SHALL go to the off hand as a light, freeing it first; a light already in the main hand SHALL move to a free off hand, else be taken out; a second weapon SHALL go to the off hand only when asked for and never beside a two-hander; otherwise the main hand's item SHALL be taken out.

#### Scenario: A torch and a sword
- **WHEN** a sword is taken while a torch is in the main hand and the off hand is free, then a torch is taken beside a sword
- **THEN** the first torch moves to the off hand and the sword goes to the main hand; the second torch goes to the off hand as a light

### Requirement: Two-handed items and taking out
A two-handed item SHALL free the off hand, then the main hand. Taking an item out SHALL read and clear its hand's record, and a dropped first weapon SHALL hand the main hand to an off-hand weapon or light.

#### Scenario: Dropping the sword
- **WHEN** the first weapon is dropped while the off hand holds a dagger
- **THEN** the record's main hand holds the dagger and the off hand is empty

### Requirement: Loose items and cores
A held item coming loose SHALL clear every out hand that is not a shield and its first held entry, and SHALL clear a stowed hand whose item is no longer listed. A race flagged 0x2000 SHALL take only the AL power crystal as its core, one flagged 0x1000 only crystal A, and only while it wears none of the five power crystals.

#### Scenario: A golem's core
- **WHEN** a body of race flags 0x1000 is given crystal A, crystal AL, then crystal A while wearing crystal C
- **THEN** it takes the first and refuses the other two

### Requirement: Possessions from the record
A character's possessions SHALL start from its record: the profession word with its second-set flag 0x0100_0000, the four held items with their flags words (the out set's main and off hand, then the stowed set's), the worn items, and the carried list with each entry's position in sixteenths of a unit.

#### Scenario: A lit torch in the off hand
- **WHEN** a record holds items 1 to 4 in its hands with the off hand's flags 0x0200_0000, the second-set flag in its profession word, and a held item listed at 40 by 4
- **THEN** the out set holds 1 and a lit 2, the stowed set 3 and 4, the second set is out, and the entry sits at 40 by 4

### Requirement: Doll weapon take-out
Taking a weapon out of a hand SHALL read and, when the job records, clear the record entry naming it in either hand. A scheduled take-out of the main hand whose weapon was the body's first attached weapon SHALL move the off hand's light or weapon into the record's main hand with the off hand's flags; a shield SHALL stay.

#### Scenario: Promotion after the first weapon only
- **WHEN** a scheduled take-out removes the main hand's sword while the off hand holds a dagger, once with the sword the first attached weapon and once with the dagger attached before it
- **THEN** the first leaves the dagger in the record's main hand, the second leaves it in the off hand

### Requirement: Doll off-hand take-out
Taking out the off hand's light or shield SHALL read and clear the record's off-hand entry without comparing its item. A hand that already holds the item the take-out makes room for SHALL be left alone.

#### Scenario: A shield replaces a two-hander
- **WHEN** a shield is taken while the off hand's model is a two-hander named in the record's main hand
- **THEN** the main hand's entry is cleared and the off hand holds the shield

### Requirement: Swapped sets dress the hands
After a weapon-set swap that changes a hand, the moveset's hands SHALL follow the record's out set: the main hand's weapon, a second weapon or shield in the off hand, and an off-hand item with a light section as the off hand's light. A carried torch outside the record SHALL stay in its hand.

#### Scenario: The torch survives a swap
- **WHEN** a body carrying a torch in its main hand swaps to a set holding a sword and a shield
- **THEN** its main hand keeps the torch and its off hand holds the shield

### Requirement: Re-dress apply order
Applying a re-dress plan SHALL detach each changed hand whose body model is present, then clear the body's weapon part once when its weapon class word has none of the `0x30` bits, then attach the new models, then refresh the body when both target item ids the plan read are 0, then recompute the body's grip, then restart the stance unless the body is in its swap state, clear the attach flag and, for a swap plan, exchange the sets.

#### Scenario: An empty stowed set on a body without weapon bits
- **WHEN** a swap plan whose targets are both 0 changes the main hand of a body holding no models, in its swap state with class word 0x40
- **THEN** the steps are clear the weapon part, refresh the body, recompute the grip, clear the attach flag and exchange the sets

### Requirement: Doll hand scene
The doll's hand-scene builder SHALL mark every hand whose target item differs from the item it shows as changed and SHALL build a model only for a non-zero modelled target, attached as a weapon when the item has a weapon section (as a light when it is the off hand and the item also lights), else as a shield, else as a light, else as a weapon. A weapon-section node SHALL settle at -1 and a shield's at the largest word.

#### Scenario: A shield joins the off hand
- **WHEN** the main hand keeps item 5 and the off hand's target becomes a shield
- **THEN** only the off hand changes and it is attached as a shield whose node settles at 0x7FFFFFFF

### Requirement: Hand take-in record flags
A weapon or light taken into a hand SHALL write only the top bit of the dragged item's flags into the record; a shield SHALL keep all its flags.

#### Scenario: A sword with flags 0x82000000
- **WHEN** a sword flagged 0x82000000 is taken in while a torch is the main hand
- **THEN** the record's entry is written with 0x80000000

### Requirement: Released hand model
Releasing a hand's old model SHALL detach its scene node, SHALL refresh the body when both record hands name no item, and SHALL restart the stance when the take-out job's mode has bit 1.

#### Scenario: Scheduled take-out
- **WHEN** a scheduled take-out (mode 7) empties both hands
- **THEN** the node is detached, the body refreshed and the stance restarted

### Requirement: Light toggle reads the held part
The light toggle SHALL read whether the light burns from the first held item's part mark, which a weapon or light take-in sets from the item's full flags, and SHALL fall back to the lit bit of the record hand holding that item when the part carries no mark of its own; a successful toggle SHALL turn the mark over.

#### Scenario: A lit torch dropped onto the doll
- **WHEN** a torch flagged 0x2000000 is dropped onto the doll (its record entry written with only the top bit) and the light is toggled
- **THEN** the light goes out: the record's lit bit stays clear and the next toggle sets it

- **WHEN** a swap plan whose stowed set is empty changes the main hand of a body in its swap state with class word 0x40
- **THEN** the steps are detach main, clear the weapon part, refresh the body, clear the attach flag and exchange the sets

### Requirement: Model attach job
Attaching a weapon's model SHALL light its part when the item flags carry 0x2000000, mark its meshes to cast shadows when the object record answers the 0x100 test, clear the body's weapon part when the class word has no 0x30 bits, and leave the take-in job. Attaching a shield SHALL clear the part on the same test, never light or mark shadows, and leave the node finaliser.

#### Scenario: Weapon and shield
- **WHEN** a weapon with flags 0x2000000 is attached to a body of class 0x10, and a shield to a body of class 0
- **THEN** the weapon lights, keeps the part and leaves the take-in; the shield clears the part and leaves the node finaliser

### Requirement: Take-in wait
The take-in job SHALL stay queued until the attached node carries 0x2000000 and its part 0x10000. A scheduled take-out SHALL exist only for a hand whose model slot is filled.

#### Scenario: Part not marked
- **WHEN** the node carries 0x2000000 but its part only 0xFFFF
- **THEN** the take-in job is not ready and stays queued

#### Scenario: Empty hand
- **WHEN** a take-out is scheduled for a hand with no model
- **THEN** none is made

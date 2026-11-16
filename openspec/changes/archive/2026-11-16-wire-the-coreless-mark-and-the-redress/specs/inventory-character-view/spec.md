## ADDED Requirements

### Requirement: Core drop clears the coreless mark
A core crystal dropped on the figure and shown SHALL clear the coreless mark of the world body the view stands for, without the crystal entering the record's lists, and no armour pass SHALL run for that drop.

#### Scenario: The AL crystal on a coreless golem
- **WHEN** an AL crystal is dropped from the pack onto the figure of a golem marked coreless
- **THEN** the golem's mark is clear, and stays clear after the pack's change

### Requirement: Shown core take-off marks the golem
A release of a drag of the shown item SHALL set the world body's coreless mark, fade its skin glow to 0 and start its collapse unless its brain rests as a statue.

#### Scenario: The core dragged off an awake golem
- **WHEN** the shown AL crystal is dragged off an awake golem's figure into the pack
- **THEN** the golem is marked coreless and collapses

#### Scenario: The core dragged off a statue
- **WHEN** the shown core is dragged off a golem resting as a statue
- **THEN** the golem is marked coreless and does not collapse

### Requirement: Loot equipment core removal
An item dragged off a body's loot equipment panel SHALL mark the body coreless, fade its glow and start its collapse (unless it rests as a statue) when the body, without the item, takes it as its core.

#### Scenario: The AL crystal off a golem's equipment
- **WHEN** the AL crystal leaves the equipment of a golem wearing no other crystal
- **THEN** the golem is marked coreless

### Requirement: Armour pass on equipment changes
The armour pass SHALL run again only when a body's worn list or the items of its out set changed; a change of its carried list alone SHALL run none.

#### Scenario: A crystal taken from the pack
- **WHEN** a crystal leaves a body's carried list and nothing worn or held changes
- **THEN** the body's rule bits stay as they were

### Requirement: Re-dress at the loadout close
The loadout form's close on the roster or a specialist's page SHALL re-dress the player and every body whose record word carries a company role; a close on the hire or event page SHALL re-dress none.

#### Scenario: Roster page closed
- **WHEN** the loadout form closes on the roster page
- **THEN** the player's and the merchant's hands show their records' out sets, and a body without a role is untouched

### Requirement: Re-dress after a trade
A confirmed trade at the hub SHALL re-dress the player.

#### Scenario: A trade confirmed
- **WHEN** the player confirms a trade
- **THEN** the player's hands show its record's out set

### Requirement: Re-dress rules
A re-dress SHALL apply the underwear rule to the worn list, then make the body's hands and its main hand's model show the record's out set, keeping a carried torch.

#### Scenario: Hands that differ from the record
- **WHEN** a body whose hands hold a weapon its record's out set does not name is re-dressed
- **THEN** its hands follow the out set

### Requirement: Equipment finaliser order
A body's own equipment finaliser SHALL clear the body's hand effects, rebuild its apparel, refresh its effects, refresh the player's light only when the body is the player, then sync its view; a view's finaliser SHALL refresh the player's light whatever its body, after its callback and the world body's sync.

#### Scenario: Another body's equipment change
- **WHEN** a body other than the player puts on an item that sees in the dark
- **THEN** the player's light is not switched

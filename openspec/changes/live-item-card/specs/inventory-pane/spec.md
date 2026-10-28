## ADDED Requirements

### Requirement: Live description card
A right press on an item panel entry SHALL request that item's description card through the panel's mouse decision, build it from the record's card facts, and show it until the pointer leaves it or the last item panel closes; a card whose name does not resolve SHALL not be shown.

#### Scenario: Inherited name
- **WHEN** a record's model name and description ids are both `0x80000000` and its parent record names "Mail Coat" and "Rings of iron."
- **THEN** the card facts carry that name and description

#### Scenario: Unresolved name
- **WHEN** a record's name id inherits from a parent that does not exist
- **THEN** the card facts are not titled and no card is built

### Requirement: Card page bytes
The card's title tint byte, rank byte and socket flag SHALL come from the section of the page the card shows: a weapon section without the withheld flag, else an apparel body of armour type 1 to 6, else a shield section without the withheld flag.

#### Scenario: Withheld weapon on an apparel record
- **WHEN** a record has a withheld weapon section with quality 9 and an apparel body of armour type 3 with appearance quality 77 and rank byte 0x21
- **THEN** the card reads condition 77 and rank 0x21 and no socket flag

### Requirement: Card icon key
The description card SHALL acquire its icon with the entry flags masked to their low 28 bits, or with the apparel framing bit when the model flags have `0x1000`, keyed without the streamed or effect bits.

#### Scenario: Lit model
- **WHEN** a lit model's card is acquired with flags `0x40AB0003`
- **THEN** the slot's key is `0x00AB0000` with the lit bit

### Requirement: Panel icon release
Closing an inventory or loot window SHALL release one reference for every entry icon it held; the last reference SHALL free the slot's atlas cell and id, and the next acquire SHALL reuse a freed slot before growing the cache.

#### Scenario: Reuse after release
- **WHEN** a slot's only reference is released and another item is acquired
- **THEN** the new item takes that slot and the cache keeps its length

### Requirement: Encumbrance after an equipment change
A body whose possessions change after its first frame SHALL have its encumbrance re-summed from its worn burdens, the shield of its first two hands and its techniques, looking ids up by their low 30 bits, into both its physical and its fighter stats.

#### Scenario: Worn item and shield
- **WHEN** a body puts on an item of encumbrance byte 10 and holds a shield of weight byte 100, block 0 and bulk 100 without techniques
- **THEN** its encumbrance becomes 0.525

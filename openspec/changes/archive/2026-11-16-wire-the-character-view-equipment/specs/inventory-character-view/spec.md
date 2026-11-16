## ADDED Requirements

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
The open arsenal SHALL take each pressed key (Escape closes, the bound inventory key flips the embedded inventory) and a right release over its form, which closes it.

#### Scenario: Right release over the arsenal
- **WHEN** the player releases the right button over the open arsenal
- **THEN** the arsenal closes and the player's controls come back

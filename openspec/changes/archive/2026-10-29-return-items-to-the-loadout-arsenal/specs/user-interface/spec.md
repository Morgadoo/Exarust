## ADDED Requirements

### Requirement: Loadout arsenal return
For an item dropped back onto its arsenal outside a champion's form, the loadout editor SHALL clear the form's mask bit on the item's first arsenal entry having it when the item is of the form's category, then sort the item into the kit container it left: from the weapon list by the drag's off-hand mark, from the worn list as worn, otherwise the weapon list's first entry with the item (which leaves), then a worn entry no hand holds.

#### Scenario: A lent item comes back
- **WHEN** an item of the form's category is dropped on the arsenal and its arsenal entry has the mask bit
- **THEN** the bit is cleared and the drag drops its item

### Requirement: Loadout arsenal return count
The editor SHALL count the copies the kit still holds after the sort. An off-hand copy SHALL remove the first worn entry of the item that a hand holds and count the other worn entries. Outside match mode a worn copy SHALL count the worn entries less one plus the plain weapon entries, and a weapon the plain weapon entries less one plus every worn entry. A drag from the preview SHALL remove the first unheld worn entry (or off-hand weapon entry) instead.

#### Scenario: An off-hand copy returned
- **WHEN** the worn list holds the item unheld once and held twice and its off-hand copy is dropped on the arsenal from the weapon list
- **THEN** the first held entry leaves and the count is 2

#### Scenario: A weapon returned from the weapon list
- **WHEN** the weapon list holds the item plain twice and as an off-hand copy once, the worn list once, and a plain copy is dropped on the arsenal from the weapon list
- **THEN** the count is 2 and the weapon list keeps its three entries

### Requirement: Loadout gate copies
The loadout editor's equip gate SHALL count the copies the weapon and worn containers hold, not the arsenal's.

#### Scenario: The gate ignores the arsenal
- **WHEN** the kit holds two copies of an item, both held, and the arsenal a third
- **THEN** the item may not be equipped again

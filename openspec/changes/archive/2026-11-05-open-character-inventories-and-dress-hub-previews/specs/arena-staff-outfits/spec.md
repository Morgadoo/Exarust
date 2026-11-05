# Spec Delta

## MODIFIED Requirements

### Requirement: Loadout form drags
A left press over an entry of the loadout form's kit list or arsenal box SHALL start its drag, its icon following the pointer. Released over the preview, an arsenal entry SHALL be taken as the equip rules say and a kit-list entry SHALL dress the preview; released over the arsenal box, a kit-list entry SHALL be returned; released over its own view, the entry SHALL only move there; released anywhere else, it SHALL stay where it was.

#### Scenario: Equip by drag
- **WHEN** a rank 0 weapon is dragged from the arsenal box onto the preview at tab 0
- **THEN** the entry is lent and the kit list holds the weapon

#### Scenario: A drag let go elsewhere
- **WHEN** an arsenal entry is released over the form's records
- **THEN** nothing is taken and the entry keeps its place and flags

## ADDED Requirements

### Requirement: Loadout dress from the kit list
A kit-list item released over the preview SHALL be refused when the kit list holds no more copies of it than the shown record's two out hands hold, and otherwise by the rank and points gate; a passing one SHALL be worn (after the equip conflict pass, whose displaced pieces stay in the kit list) or taken into the hands in the tab's loadout, and the kit list SHALL keep its entry.

#### Scenario: One copy already held
- **WHEN** the kit list holds one sword and the tab's loadout holds it in the main hand
- **THEN** dragging the sword onto the preview is refused

### Requirement: Loadout preview take-off
A left press over the preview's figure SHALL pick the item under the pointer and drag it. Released over the kit list, the item SHALL come off the tab's loadout (a worn piece with the underwear rule after it, a held item by the scheduled take-out) and the kit list SHALL keep its copies. Released anywhere but the kit list and the arsenal box, nothing SHALL change.

#### Scenario: A shirt off into the kit list
- **WHEN** the worn shirt is dragged off the preview onto the kit list
- **THEN** the tab's loadout no longer wears it and the kit list still holds it

### Requirement: Loadout preview return
A drag off the preview released over the arsenal box SHALL have the arsenal take the item back as for a return, the kit list lose its first copy, and the item come off the tab's loadout; then every loadout SHALL keep at most the kit list's remaining copies less one in each hand pair (none when the list held none).

#### Scenario: Back to the arsenal
- **WHEN** a held sword with two kit-list copies is dragged off the preview onto its shelf's box that lent it
- **THEN** the lent mark clears, the kit list holds one copy and no loadout keeps the sword in a hand pair that held it once

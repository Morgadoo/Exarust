# inventory-pane Specification

## Purpose
The inventory pane: where an icon sits, when a drop is refused, and which icon a point hits.

## Requirements

### Requirement: Drop
A drop from outside SHALL place the icon at the cursor minus the grab, clamp it inside the pane, and add its bulk. A drop from the same pane SHALL only move that icon. An item larger than the pane's limit SHALL be refused.

#### Scenario: Place and move
- **WHEN** an icon of bulk 2 is dropped from outside near the far corner of a 100 by 80 pane, then dropped again from the same pane
- **THEN** it is clamped inside, the bulk stays 2, and the second drop only changes its position

### Requirement: Capacity
A drop from outside SHALL be refused when the bulk would pass the capacity. A drop from the linked pane SHALL not be refused for capacity. A container SHALL NOT be dropped onto the pane that shows it.

#### Scenario: Linked and full
- **WHEN** a bulk of 2 is dropped into a pane of capacity 1 from the linked pane, then a bulk of 1 is dropped from outside, then the pane's own container is dropped
- **THEN** the linked drop lands, the outside drop is refused, and the container is refused

### Requirement: Drag and hit
Ending a drag SHALL remove the icon. The hit SHALL be the top-most icon whose rectangle holds the point.

#### Scenario: Top icon
- **WHEN** two overlapping icons are dropped and the point lies in both, then the lower icon's drag ends
- **THEN** the hit was the upper icon, and the lower icon is gone

### Requirement: Marked drops
A pane SHALL refuse an item that shares a bit with its reject mask. It SHALL refuse an item whose flags, masked by the required mask, are not the required value. An own-only pane SHALL accept only an id it already lists with the low bit set. A limit of 0 SHALL not refuse for size. A capacity of 0 SHALL not refuse for bulk.

#### Scenario: A mark, a list, and a zero limit
- **WHEN** a marked item is dropped, an item without the required bit is dropped, an unlisted id is dropped into an own-only pane, a listed id with the low bit is dropped, and a large item is dropped into a limit of 0 and a capacity of 0
- **THEN** the marked item is refused, the missing bit is refused, the unlisted id is refused, the listed id lands, and the large item lands

### Requirement: Item card page
An item card SHALL be absent for an id below 1, a container, a missing model, and a model mark of 0. A map SHALL be a map and have no value. Otherwise the value SHALL be the coin word divided by 4, unless the high bit is set, which keeps the word. A weapon that is not withheld SHALL be the weapon page. An armour type from 1 through 6 SHALL be the apparel page. A shield that is not withheld SHALL be the shield page. Any other item SHALL be the plain page.

#### Scenario: A weapon, a coat, a shield, and a map
- **WHEN** an id of 0, a container, a map, a missing model, and a mark of 0 are chosen, a weapon of 7 coins is chosen, the same weapon keeps its full word, a withheld weapon of armour 6, 0, and 7 is chosen, and a plain item of 3 coins is chosen
- **THEN** the first five have no stats value, the map is a map, the weapon value is 1 and then 7, armour 6 is apparel, armour 0 is a shield, armour 7 is plain, and 3 coins print 0

### Requirement: Weapon bars
A weapon card SHALL show a slash, crush, pierce, or thrust row only when that byte is above 5. The shown bar SHALL be that byte times 0.05.

#### Scenario: Five stays hidden and six is shown
- **WHEN** slash is 5, crush is 6, pierce is 0, and thrust is 20
- **THEN** slash and pierce are absent, crush is 0.3, and thrust is 1

## ADDED Requirements

### Requirement: Dormant popup menu
The popup menu SHALL lay out items six units tall and separators three units tall inside two units of padding, as wide as its widest caption with its shortcut label plus sixteen units. A shortcut label SHALL read the held modifiers as "Shift+", "Ctrl+" and "Alt+" in that order followed by the key's system name. No game screen SHALL create a popup menu.

#### Scenario: A shortcut label
- **WHEN** an item's shortcut is Ctrl with the O key
- **THEN** its label reads "Ctrl+O" and ends six units short of the menu's client right

### Requirement: Dormant popup menu chain
Hovering a menu item SHALL open its submenu at once beside it, flipped at the screen edges, and a press on an enabled item with an action SHALL run the action and close the whole chain.

#### Scenario: A press closes the chain
- **WHEN** an enabled item with an action is pressed in a submenu
- **THEN** the action runs and every menu of the chain closes

#### Scenario: A submenu opens on hover
- **WHEN** the pointer moves onto an enabled item with a submenu
- **THEN** the submenu opens at once at the menu's right edge level with the item

### Requirement: Dormant tree view
The tree view SHALL list every node in pre-order, one row each, indented by an icon's width per level. Expanding a leaf SHALL ask the owner for its children and collapsing a node SHALL release them. A press on the selected row or on a row's chevron SHALL toggle that row. Keys SHALL step through the rows clamped to one before the row count, which counts a blank row after the last node except in popup mode. No game screen SHALL create a tree view.

#### Scenario: Collapsing with a chevron
- **WHEN** the chevron of an expanded root is pressed
- **THEN** its descendants are released, children before their parents, and the row count drops by their number

#### Scenario: The popup's last row
- **WHEN** the last node is selected in popup mode and Down is pressed
- **THEN** the selection stays where it is

### Requirement: Dormant object appearance editor
The object appearance editor's handlers SHALL write the look's levels at 255 steps, pack three variation steps into one byte, edit the selected layer's amount, switches, material, variant and colour, and leave the look alone while the editor fills its own controls. No game screen SHALL open the editor.

#### Scenario: A variation step
- **WHEN** the first variation slider is set to 0.6
- **THEN** the label reads "Var 2.0" and the byte's low three bits hold 3

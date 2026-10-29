## Context

The controls follow the existing widget modules (`table_view`, `item_view`,
`drop_down`): plain state with methods that return replies and events, and
draws returned as positions and palette slots rather than painted.

## Decisions

- A menu chain is one `MenuChain` holding the open levels; the original's
  parent and child pointers become levels, its pointer capture the
  `captured` level.
- Submenus are built from item lists and laid out before they are placed
  (the original's creator callback is the owner's; when it lays the menu
  out is not established).
- Tree nodes are nested vectors addressed by paths; the original's node
  pointers become paths. A collapse that releases the selected node drops
  the selection (the original keeps a dangling pointer).
- Key names: the original asks Windows for the name of the key's scan code
  without the extended bit; Exarust carries the US layout's names.
- The appearance editor's lists take their ids from the caller; the
  material tables are not carried.

## Open questions

- What the item state setters the level and balance controls call do
  (outside the editor's unit): left as events.
- When the submenu creator lays out the new menu: Exarust lays it out first.

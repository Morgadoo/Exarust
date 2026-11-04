# Design

## The form's item views

The constructor places the kit list at (4, 180), 228 wide and 492 times
the wide-interface scale tall, with mask 0x200; the arsenal box 236 wide,
628 (548 wide) tall at y 88, four pixels from the form's right edge, with
mask 0x400; and the character view at x 236, the form's width less
2 × 236 + 8 wide, its own height less 4. The box view is rebuilt from the
arsenal's shelf and box; the kit list from the hub stash (the manager) or
the employee's own list.

## Rules

- Equip: the arsenal container's drag start lends the entry (0x400 on the
  roster page); the character view's accept refuses an entry sharing the
  page filter, then compares the item's rank with the tab's (5 for none)
  and the loadout cost plus the item's with twice the rank's limit; its
  drop adds the item to the kit list. The view's change stores the
  record's outfit into the tab's loadout.
- Return: the arsenal container's drop clears the lend from the shown
  box's first lent entry when the item's shelf is shown, else asks the
  arsenal's lend query; an item neither took back joins the shown box
  (same shelf) or is stowed with no flags; then every loadout keeps at
  most as many hand copies as the kit list still holds and loses its first
  worn copy.

## Open questions

- The container's highlight word: taken as the yellow mask [guess].
- The lists' stored positions and the box sort: the item panel's grid
  pass lays the entries out [guess].
- The floor scene's lights do not reach the preview inside a level; the
  view keeps its stand-in light [deviation].

# Design

The original's drag-over slot (a `0xFB01` item drag) answers in this order:
- the owner's accept callback (none in the inventory or the trade window);
- apparel: the view allows changes (the loadout form's page 3 view does not)
  and the race's look word has bit 4;
- weapon: section flag `0x2000000` needs the race kind word's bit `0x10000`,
  else flag `0x4000000` needs bit `0x20000`, else it passes;
- shield: flag `0x2000000` needs bit `0x10000`;
- any other item passes only as the AL crystal (500), on a race whose flag word
  has `0x2000`, while the view shows no item.

A refused item never reaches the view's drop. Exarust delivers the drop first,
so the test runs at its start and cancels the drag, which returns the item to
its source as the original's refused drag does.

In 0.9.5, ten weapons carry `0x2000000` and only the four golem races with
kind bit `0x10000` take them. No weapon carries `0x4000000` without
`0x2000000`, so the large-race bit never decides alone. No shield carries
`0x2000000`.

Not done:
- the core crystal's drop (shown item, mark clear) still cancels. Only a
  golem's window could reach it.
- the loadout form's page 3 view lock is not modelled.
- the background wash belongs to the creator's view only (the inventory and
  the trade window set the view's transparent flag), so it stays unwired.

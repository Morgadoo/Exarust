## Context

The original's loadout form has three item containers: the arsenal on the left (the constructor's 228-wide container), the company stock on the right (the 2814-byte handler on its drop-received slot, catalogue ids and the form's lend mask) and an optional worn container whose entries mark a hand-held copy with 0x100; the weapon container marks an off-hand copy with the top bit. Exarust's editor keeps `weapons` (top bit), `armour` (0x100) and `arsenal` (the company stock, with the mask), so the handler maps onto those lists.

## Decisions

- **A separate function.** The handler is `return_to_stock`; `apply_drop` stays the list model's equip (the containers' own drag and drop is outside the form) so the running screen keeps its press model.
- **External answers are parameters and fields.** The arsenal's lent query is the `lent` argument; its quantity write is `zero_stock`; the record's hand-slot clearing is `freed_hand`; the count written to the rank's record is `count`. In match mode the count is the one the off-hand branch left (the original writes it before the match branch walks the weapon list).
- **`DropSource::Preview`** is added so the preview-only removals can be expressed.

## Risks

- The handler is not called by the running game (nothing opens the loadout screen), so there is no play regression; the restored gate count changes when a second copy may be equipped from the arsenal in the list model.

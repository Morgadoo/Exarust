## Why

Roadmap tasks B1 and B4 (lane `gamegui2`): the in-game GUI unit's open
rows. The inventory's Primary and Secondary page buttons had no callback,
the journal's entry list never dropped down (its selection kept no row or
titles), and the controls' idle turn did not ask whether an in-game screen
was open. Two ledger readings were wrong: the page buttons' callback was
read as a rescale of the character view, and the unit's skin setter as a
GUI unit, where it stores a pointer slot.

## Evidence

REA (Exanima.exe 0.9.5), read in the assembly:

- The inventory's record table carries the same record callback on its
  two page-button records ("Primary", "Secondary"): when the reporting
  record's state has either 0x60 bit it calls the weapon-set swap of the
  character the inventory shows, the same routine the swap key calls (busy
  state bits 0x10202 or a drag in flight refuse; with the character's
  inventory open the sets swap at once, else the switch motion is asked).
- The journal's selection writes the list record's caption, its 1-based
  row, the entry count and the title array; the list's record callback,
  not editing (state bit 0x200000 clear), stores the text box's text into
  the selected entry and selects the record's row less one; editing, it
  renames the entry.
- The controls' idle branch, once the clock is past the stamp plus
  10 000 ms, asks whether any child of the active task has a tag in the
  0xce100000 family; with one, the stamp is retaken and nothing turns.
- The skin setter's two callers pass −2 (the controls' start) and 0 (the
  game's end); the value is the pointer slot the guitex and menu-cursor
  skins and every new inventory character view show (the viewport's
  pointer query answers it), a negative slot hiding the system pointer.
- The unit init's control classes 0xac2, 0xac3 and 0xac4: no record
  naming them is reachable (confirmed: their only records are the
  unreferenced table and the technique selector's own, whose opener is
  reached only from the power slot).

## What Changes

- The inventory's page buttons report; the one turned on asks the shown
  body for the set swap.
- The journal keeps its list's row and titles; a release over the list
  opens its drop-down list, and a pick selects that entry. The hub's
  drop-down opener, pointer drive and paint are shared with the journal.
- The idle turn restamps its clock while a game screen is open.
- The skin setter is renamed after what it holds (the pointer slot).

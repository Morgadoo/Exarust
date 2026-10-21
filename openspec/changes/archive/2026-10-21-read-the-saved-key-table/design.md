# Design

`Bindings` keeps the slot table the game reads (virtual-key codes); the
saved table exists only in the payload. `decode` starts from the shipped
saved table (`DEFAULT_ROWS`), lets the payload's count overwrite its top
rows, clears row 21 and converts every row to its slot (`ROW_SLOTS`,
`slot_code`); slot 0, the primary mouse button, has no row and keeps its
shipped code. `encode` writes each row from its slot through `row_code`.

The conversion's keyboard half is the system's scan-to-key mapping (the
"extended" form, which gives left and right modifiers their own codes).
Exarust has no system call to make, so it keeps a US-layout table; the
shipped tables pin every shipped key, including the arrows, whose extended
scan codes lose the extended bit before the mapping.

## Open questions and how they were answered

- Which word picks the older shape: the version (above 100 has the flag),
  from the loader's assembly.
- The count: any count is copied, with no bound, from the loader's loop;
  Exarust drops rows past 23 rather than overwrite anything.
- Slot 21: it is the saved table's row 21 the loader clears, and no slot
  reads that row (the conversion's 22 copies skip it); the slot table's slot
  21 (next power) comes from row 19.
- How the game reads a one-sided modifier code: not traced; Exarust reads
  it on its own side [guess, by the codes' meaning].

No format page changes; the page extended is `docs/PLAYER.md` (Bindings).

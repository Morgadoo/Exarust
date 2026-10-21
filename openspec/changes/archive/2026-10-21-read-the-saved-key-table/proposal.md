# Proposal

## Why

The original does not save its 23-slot virtual-key table. It saves a second
table, the Controls page's rows in keyboard scan codes, and converts it into
the slots after reading it. Exarust wrote and read the slot table itself, so a
payload saved by the original (or one Exarust wrote) put its keys in the
wrong slots. The reader also chose the older shape by the count rather than
by the version word, refused any count but 23, and did not clear the row no
slot reads.

## What Changes

- The payload's rows are the saved table, in the Controls page's order, with
  keyboard scan codes (bit `0x100` for an extended key) and `0x8000` plus a
  mouse button's virtual-key code; row 21 fills no slot.
- Loading follows the original's reader: a version above 100 has the
  controller flag then the count, any other version the count and no flag;
  the count's rows overwrite the shipped saved table from the top; row 21 is
  cleared; each row becomes its slot through the original's conversion (a
  code with a `0xC000` bit keeps its low 14 bits, any other its low byte's
  scan code mapped to a virtual-key code, so the modifiers become their left
  keys' one-sided codes).
- Saving writes version 101, the flag, the count 23 and the rows from the
  slots (the inverse conversion; row 21 is 0).
- The one-sided modifier codes read their own side.

## Capabilities

### New Capabilities

### Modified Capabilities

- `input-bindings`: the binding file's rows, its reader and its writer.

## Impact

- `exa_game::bindings` (`Bindings::decode`/`encode`, `ROW_SLOTS`,
  `DEFAULT_ROWS`, `slot_code`, `row_code`, `key_code` for the sided codes).
- Docs: `docs/PLAYER.md` (Bindings). `docs/PLAN.md` is the integrator's.
- Phase 6 (player and input), the hardware configuration's Controls page.
- Evidence: REA (Ghidra 12.1.4) on the bindings loader and saver, the
  saved-to-slot conversion and its per-code helper, and the two shipped
  tables' bytes, which convert into each other row for row.
- Out of scope: the system's scan-to-key mapping beyond a US layout (Exarust
  keeps a table, a guess outside the shipped keys); the Controls page's
  rebinding, which writes virtual-key codes into the slots as before; the
  memory a count above 23 overwrites in the original (Exarust drops those
  rows).

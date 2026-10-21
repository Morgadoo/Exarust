# Design

Format page: [`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md),
Object database.

## Model

A `Store` is one tier: the table entries, the 16-byte slot after them and
the blob, exactly as the original's database object holds them. Records are
views into the blob (`Record`); changes copy a payload, edit it and store it
back through the original's payload store, so offsets, holes and order
evolve as in the game. `ObjectDb` holds the three stores and three
libraries and carries the routines that cross tiers. The routines keep the
original's return conventions (an unchanged id, 0) instead of options where
callers depend on them.

## Faithful quirks kept

- The loader neither sorts nor compacts; ids compare by 32-bit difference.
- A deletion leaves the deleted entry or the last entry in the slot after
  the table, which the writer saves.
- Duplication reads the source's flags by its index after the insertion.
- Instancing compares the copy's description with its source's, so it
  always inherits.
- The resolver succeeds with an empty string for a chosen id without text.
- Removal releases every model id whose low 31 bits are set, own or not.
- The library's release wraps a count of 0; its free hint only rises.
- Section resize grows without updating the size word.
- Packed setters OR unmasked values in; the signed setter's magnitude
  spills into the sign bit.

## Guards (where the original would misbehave)

- Section walks stop at a size below 8 or past the payload (the original
  loops or reads past it).
- Entries and strings outside their blob or text have no view.
- Promotion stops 64 containers deep (the original recurses without limit).
- Table bytes the original reads past the end of its tables (old operative
  classes above 11, layer materials above 51, material sets 0 and above 15)
  give 0 or keep the value.
- The slot after a grown table is written as zeros (the original writes
  bytes past its allocation).
- An unknown magic is an error (the original logs and empties the
  database).

## Open questions

- The meanings of the item-use codes and the quantised fields' records:
  named by position.
  Answer with REA on their readers (item screen, trade, randomisers) when
  those are ported.
- The body converter of the oldest layout (3014 bytes): REA when a file of
  that layout turns up; none ships.

## Tests

Hand-worked unit tests for every routine on synthetic records; the
installed databases: `objdb.rdb` (counts, flags rule, strings, containers,
folders), every level's database and library (byte-exact write-back, own
strings, a full migrate and promote pass staying total), every actor
database (upgrade and write-back), and every shipped library.

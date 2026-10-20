# Design

## Behaviour

Conversion is false unless the mode is 4 and the pending count is zero. A legacy entry alone converts. Without it, the record must have been read and its tag at `+8` must differ from `0xdbac0d06`; the same tag does not convert. A missing entry or a failed read does not convert.

The record table writes the count at `0x4f0550` as a dword, then for each of the count records at `0x4f0560`, stride 24, the kind at `+0`, the qword at `+8` and the qword at `+0x10`.

## Model

`save_needs_conversion(mode, pending, legacy, entry, read, tag)` returns the decision. `write_save_records(&[SaveRecord])` returns the count and records in wire order.

## Open questions

The archive and stream routines are not in.

## Docs

These methods are 180 and 146 bytes. `XA_SaveGames` moves from 5% to about 6%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

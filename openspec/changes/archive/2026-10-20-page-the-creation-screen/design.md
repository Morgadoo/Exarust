# Design

## Behaviour

Paging compares the maximum, the global object's count less one, with the index at `0x254`. When the maximum is at or below the index nothing happens. Otherwise the mode at `0x274` becomes 1 and the refresh runs with the index plus one.

Selecting computes the wanted index less one and compares it with the child's `0x254`. When equal nothing happens. Otherwise the child's mode becomes 1 and the refresh runs with the new index.

## Model

`page_creation_next(max, index)` and `select_creation_index(wanted, current)` return the mode, the index and the refresh flag. An unchanged outcome returns mode 0.

## Open questions

The refresh body is not in.

## Docs

These methods are 95 and 61 bytes. `SG_Character_Create` moves from 27% to about 28%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

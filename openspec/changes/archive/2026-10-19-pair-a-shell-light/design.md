# Design

## Pair

`shell_pair` stores two floats. State 0 stores 0.1 and 0.75. State 1 under 0.25 stores one tenth of `shown * 4 * (2 - shown * 4)` and `1 - that * 0.25`. State 1 at 0.25 or above stores 0.1 and 0.75. State 2 keeps the pair. State 3 or later stores one fifth of `2 - abs(1.5 - shown * 3.5)` and `shown * 0.75`.

## Upload

`shell_index_upload` asks for `20 + 8 * channels` header bytes and `count * width` payload bytes. The target is 0x8893, the usage 0x88E4, and the access 0x88B9. `shell_index_unmap` is true only when the buffer name is above 0.

## Open questions

- The play renderer does not issue the buffer calls.

## Docs

`docs/formats/OTHER.md` gains the pair and the upload. `docs/PLAN.md` and `docs/COMPONENT-MAP.md` rate the shell mesh at 70%.

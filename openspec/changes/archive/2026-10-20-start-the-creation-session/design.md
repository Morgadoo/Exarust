# Design

## Behaviour

The table selection reads the source at `0x38`, subtracts one, stores it at the child's `0x250` and passes the entry at that index in the global table `0x4f0560` to `0x100036440` with the child's `0x248`.

Starting the session runs `0x10012da80`, stores `0x10012ec80`'s result at `0x4f0b80`, clears the byte at `0x4ef330` and schedules `0x1001246d0`.

The menu dispatch tests the byte at `0x49a300`: set takes `0x10022d3c0` with 3, clear takes `0x100029aa0` with 3.

## Model

`select_creation_table(index)`, `start_creation_session(value)` and `dispatch_creation_menu(alternate)` return the index, the session flags and the chosen call.

## Open questions

The called bodies are not in.

## Docs

These methods are 76, 59 and 60 bytes. `SG_Character_Create` moves from 28% to about 29%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

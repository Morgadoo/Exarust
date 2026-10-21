# Design

## Behaviour

The table selection reads the source, subtracts one, stores it on the child and passes the entry at that index in the global record table to the record loader with the child's record.

Starting the session runs the session setup, stores the start index's result in its global, clears the stored byte and schedules the next form.

The menu dispatch tests the resume flag byte: set takes the skin's menu call with 3, clear takes the form finish with 3.

## Model

`select_creation_table(index)`, `start_creation_session(value)` and `dispatch_creation_menu(alternate)` return the index, the session flags and the chosen call.

## Open questions

The called bodies are not in.

## Docs

These methods are 76, 59 and 60 bytes. `SG_Character_Create` moves from 28% to about 29%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

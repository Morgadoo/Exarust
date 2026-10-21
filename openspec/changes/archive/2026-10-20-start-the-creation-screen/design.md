# Design

## Behaviour

The startup writes `-1` to its sentinel, calls the global reset with `-1`, calls the input reset, clears a global, and calls the input reset again when two globals match. A missing form is created from its class.

The form is prepared through the screen setup, the attach pair with one, and the body's setup. In mode 12 the form's mode is 2 when the global threshold word is 2 and 1 otherwise; the continue button update runs, the build runs when the save-record conversion is true and the tick runs. The form and the value 2 are stored in two globals.

The step makes the two refresh calls, and when the view target exists it stores the part search's result on it. It then makes the two tick calls.

## Model

`start_creation_screen(had_form, second_reset, mode, arena_flag, build)` returns the reset, create, prepare, mode, refresh, build, tick and store flags. `apply_creation_step(has_target, part_found)` returns the link, camera, store, part and tick flags.

## Open questions

The called bodies are not in.

## Docs

These methods are 328 and 129 bytes. `SG_Character_Create` moves from 29% to about 30%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

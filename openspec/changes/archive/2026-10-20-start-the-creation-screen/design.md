# Design

## Behaviour

The startup writes `-1` to its sentinel, calls `0x1000304c0` with the global and `-1`, calls `0x10013e880`, clears `0x4f0590`, and calls `0x10013e880` again when the global at `0x4f0be0 + 0x8` equals the one at `0x49a930`. A missing form at `0x4f0370` is created through `0x100130db0` with `0x315280`.

The form is prepared through `0x1001316d0`, `0x100029470` with one, `0x100029660` and `0x10004c460` on its `0x160`. In mode 12 the form's `0x274` is 2 when the flag at `0x4f0400` is 2 and 1 otherwise; `0x10012f840` runs, the build runs when `0x10017e740` is true and `0x10002e760` ticks. The form is stored at `0x4eff60` and 2 at `0x4eff70`.

The step calls `0x1000f13c0` and `0x100133020`, and when the target at `0x180` exists it stores the part search at `0x180 + 0x180`. It then calls `0x100071a20` and `0x1001e5ea0`.

## Model

`start_creation_screen(had_form, second_reset, mode, arena_flag, build)` returns the reset, create, prepare, mode, refresh, build, tick and store flags. `apply_creation_step(has_target, part_found)` returns the link, camera, store, part and tick flags.

## Open questions

The called bodies are not in.

## Docs

These methods are 328 and 129 bytes. `SG_Character_Create` moves from 29% to about 30%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

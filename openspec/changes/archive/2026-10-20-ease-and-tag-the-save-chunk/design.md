# Design

## Behaviour

The curve is `(x × 2)² × 0.5` below 0.5. At or above it, `t = x × 2`, `a = t − 3`, `b = t − 1`, and the result is `((a × b) − 1) × −0.5`. The two branches meet at 0.5.

Writing emits the header `0xecadca01`, a twelve-byte tag chosen by a global byte (`0x2c7ba0` or `0x2c7b90`), then two dwords from the globals `0x4f2d30` and `0x2b39f0`.

Reading returns immediately unless the header matches. It then compares 1,000,000 with the tag's float at `+4` for the old flag and reads the two dwords.

## Model

`ease_save_curve(x)` is the curve. `write_save_settings(tag, first, second)` returns the header, tag kind and dwords. `read_save_settings(id, tag_value, first, second)` returns whether the header matched, the old flag and the dwords (zero when rejected).

## Open questions

The stream and file routines are not in.

## Docs

These methods are 90, 140 and 104 bytes. `XA_SaveGames` moves from 4% to about 5%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

# Design

## Behaviour

The deferred callback needs the pending byte at `0x1e4`. Without it the tick returns true. With it, the stamp at `0x1e0` plus one is compared with the clock; when it is not below the clock the tick returns true. When it is, the stamp takes the clock, the pending byte clears and the callback at slot `0x1b8` runs with `0x1c0`. The tick always returns true.

The tip release runs `0x10006e890` on `0x1d8` only when that pointer is present. The view attach stores its argument at `0x1e8` and registers `0x10012c7f0` on the view's `0x178` only when the view is present.

## Model

`flush_creation_deferred(pending, stamp, clock)` returns the fired flag, the new stamp, the new pending flag and the true result. `release_creation_tip(has_tip)` and `attach_creation_view(view)` return whether the action ran.

## Open questions

The release and registration bodies are not in.

## Docs

These methods are 81, 39 and 59 bytes. `SG_Character_Create` moves from 18% to about 19%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

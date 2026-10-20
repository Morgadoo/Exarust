# Design

## Behaviour

The tracker skips a tick when `now − stamp < 4` (wrapping). A node's window is 3000 ms when its `+0x11a4` value exceeds 1, else 5000 ms when its `+0xdf4` flags carry `0x800`, else none. With a window, the fade is `(now − seen)/window` and the node is lost once `now > seen + window`. A zero window is treated as immediately lost.

## Model

`mind_track_due(now, stamp)`, `mind_dwell_window(dwell, marked) -> Option<u32>`, `mind_dwell_fade(now, seen, window) -> (f32, bool)`.

## Open questions

The visibility test and the entry array updates are separate.

## Docs

`SG_Thaum_Mind` is **11%**. The headline stays 14.4% and 5.7%.

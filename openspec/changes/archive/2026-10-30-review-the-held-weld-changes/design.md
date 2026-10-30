## Context

The world welds are built disabled, so the slack and strength changes take effect only once something enables them; they are still reproduced so the link words match the original.

## Decisions

- `HeldWeld` records the holder condition last seen; the collapse and death changes run on a change into collapsing or dead. This matches the original's routines, which write unconditionally each time they run, while costing one comparison an item a tick.
- The groan is asked from a system on the collapse's insertion with its own roll, through the existing `PainSounded` message, so the voice's waits and gains and the sound playback are the pain voice's. One system covers every place that starts a collapse.
- The collapse writes only the slack: the limp law's smoothed length is left as it was.

## Risks

The groan's draws use their own generator rather than the original's shared one, so the exact sequence differs from retail's; the ranges and arithmetic are the original's.

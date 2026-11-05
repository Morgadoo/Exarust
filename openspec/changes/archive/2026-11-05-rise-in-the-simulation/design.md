## Context

Follows `post-frame-writes-as-commands`. No `docs/formats/` page is involved.

## Decisions

- The rise and the restore move together: the restore reads the rise's revival in the same frame. In `SimFrameStart` they run before the frame's ticks, so the ticks see a revived body with its clips and controller, as before; the frame's own systems see the rise one frame later than they did.
- The held weapon's drawn transform during the arena's load (`s4b1`–`s4b4`): `--schedule-ambiguities` lists no unordered frame writer of it next to `draw_bodies` (only the sun light's `live_environment`), and with the frame's schedules single-threaded one bridged run still shows it and another does not, while one-world runs agree. So it is not a frame-schedule ambiguity; it varies between bridged runs during the load only, with every body, grip and parent equal. Left open.

## Risks / Trade-offs

- A system later in a frame that read the risen corpse now reads it a frame later.

## Open questions

- Whether the original's per-step rise (rather than per frame) changes any observable timing: REA on the manager frame's step cadence against the AI tick's.

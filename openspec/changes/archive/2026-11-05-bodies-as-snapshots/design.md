## Context

Follows `alarm-zones-and-hotkeys-in-the-simulation`. No `docs/formats/` page is involved.

## Decisions

- The hand is not moved to the simulation: the original runs it on the main thread per GUI cursor event and writes the bodies directly (no lock, no ring). Its writes are posted instead; its reads stay in the frame.
- A posted write runs on the frame's copy first (the frame's later readers in the same frame, such as the lift reading the body it turned, see it) and again on the simulation's copy before the step. The frame's copy carries no change mark, so the next copy out replaces it with the simulation's.
- Writes to two entities at once (a door leaf and its partner's unlock) use `write_many`; a mechanism and its body use `write_pair`.
- A snapshot still crosses inward when the frame inserts it (a GUI drop's new body), so the frame can build bodies.
- Cloths stay copied in only: as snapshots, every awake cloth would be copied out each step (0.5-0.9 ms in exanima01 and exanima06).

## Risks / Trade-offs

- A frame writer of a snapshot that does not post is lost behind the bridge; the bridge warns once per type. None seen in the walk, the duel, the corpse drag, exanima01 and exanima06.
- The hand's cloth grab reads the frame's cloth, which behind the bridge holds the vertices as built.

## Open questions

- None for reverse engineering.

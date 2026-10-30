# Design

- The aura reuses the effect materials' `MaterialNode` model and the shared aura state bits; it is its own builder because the scanner's material, block and shader differ from the force aura's.
- The scan lock is an atomic word with the original's take and release; Exarust builds on one thread, so it never waits.
- The creation draws come from an MT19937 stream seeded 5489, as the other effect streams; the original's process-wide stream is clock seeded and the values are never read.
- The device latch (`exa_world::teleporters::Latch`) stays the device-side model; the bridge mirrors its writes into `Flow.story` (`reconstruction`, `checkpoint`), the words the death handler reads. In the running game no body carries a record identity yet, so both identities are 0 and a scanned death rebuilds.
- The strip clears the live body because Exarust keeps the player entity through the respawn; the original instead leaves the live body as a dressed corpse record and reloads the stripped record.

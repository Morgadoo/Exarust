## Context

The cloth stage (`exa_world::cloth`) runs once per 83 Hz tick after the bodies' tick. With hair on, fifteen hair cloths never sleep (their heads' countdowns keep them awake), and the frame falls into the four-tick catch-up, so every per-tick cost is paid four times a frame.

## Decisions

- **Write meshes once a frame.** The stage marks a stepped cloth's mesh stale; a system in the fixed main loop's after-set writes each stale mesh in place (no intermediate list) and clears the mark. The renderer extracts a changed mesh once a frame however often it changed, so the per-tick writes were repeated work; the stage no longer borrows the mesh assets mutably, so it no longer serialises against systems that read them. Exarust's own render bookkeeping: the simulation does not read the mesh.
- **Step the cloths at once.** A cloth's contacts read only the tick's shared body-part list and cell buckets, the static grid and the level, and its step writes only that cloth, so the contacts and step run in a parallel query (one cloth per batch). Everything that touches the shared object list (insert or pre-step migrate, the cell it reads, the post-step migrate) stays sequential in the list's order; the post-step migrates are applied in list order after all cloths stepped, which leaves each object in the same cell (nothing reads the order within a cell).
- **Count the split.** `ClothDebug` gains the stage's sequential parts, the per-cloth CPU time by part, the write-back's time and count, and the whole fixed tick's time (FixedFirst to FixedLast), logged in one line beside the stage's means.

## Open questions

- Where the rest of the fixed tick's hair cost goes (12.5 ms a tick against the stage's 3.8 ms, and 6.4 ms a tick with hair off): answered by the next GPU run's split line (no reverse engineering involved; nothing of the original's rules changes, and no docs/formats/ page is touched).

## Verification

- `cloth_cost_of_fifteen_hairs` (ignored bench, `--profile play`): 0.45 ms median a tick before, 0.13 ms after on the build worker; checksum `3ade39421cb4526d` both ways.
- `each_cloth_steps_as_it_would_alone`, `the_meshes_take_the_particles_once_a_frame`.

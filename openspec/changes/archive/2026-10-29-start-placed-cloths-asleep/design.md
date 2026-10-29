## Decisions

- **Positions only.** The position records reach only the cloth meshes (14 banners in exanima01), so taking them from the level's frame changes nothing else. The transform records' fix moves 1433 nodes in exanima01, and the navigation grid's corpus numbers move with it: arena by 2 columns, arenao1 by a few dozen, exanima01's start reach from 191 328 to 2713 cells. The original stamps those props too, but it treats moving props in its build differently, and Exarust's notes already list that as not reproduced. So that fix is left for a change that can look at the navigation grid. Until then the overridden props stay where Exarust has always put them, away from the banners.
- **The node frame.** `node_world` follows `spawn_model_nodes`: the placement stands for the first root, and other roots sit relative to it.
- **The countdown.** The word goes on the node's entity (`ClothStartSleep`). The cloth builder reads it from the part's node. A word under 1 is asleep (0), and any other word is the countdown. A cloth without a record keeps the constructor's 32.

## Verification

- Saved positions 10 cm in front of a half-turned placement come back 10 cm along the node's z.
- A cloth part under a node with `ClothStartSleep(0)` is built asleep. The word rule is checked for 0, −3, 1 and 40.
- The navigation corpus test is unchanged, because transform records stay local.

The GPU check is for the integrator: `EXA_CLOTH_CONTACTS=1 EXA_DEBUG_CLOTH=1 play exanima01 --perf-log 5`. All fourteen cloths should be asleep and skipped from the first second, and the stage should cost well under 1 ms. A character brushing a banner wakes it (countdown 256 at most), and it falls asleep again. If that holds, `ClothContacts` can default to on.

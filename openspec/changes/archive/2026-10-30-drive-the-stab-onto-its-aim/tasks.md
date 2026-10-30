# Tasks

- [x] Trace the duellist's reach (equip refresh, holding pass), the position score's strike ideal, and the size and scale gates with REA; compare them with `exa_ai::place` and `reach_of` (they match).
- [x] Log the held records, the attack starts and the blade contacts at debug level, and measure where the golems' stabs land on the GPU.
- [x] Trace the AI's stab-aim slot, the executor's aim start and the body tick's stab arm drive with REA.
- [x] `exa_combat::stab` (aim and drive), with tests that use the original's numbers.
- [x] Wire the drive into the swing stage and the aim into the duellist; add `Ragdoll::part_centre`; move the quick arcsine into `exa_anim`.
- [x] Corpus check: the golem halberd's record from the installation (`@GlmHalbrd A`: 175.9 cm, sweet spot 143.0 cm, reach 227 cm at size 1.4), read at the equip in the GPU run.
- [x] GPU runs: `exanima06` (the golems' stabs land at the point with non-zero impact); `exanima01 --opponent` with a sword and with a spear.
- [x] docs/PLAN.md and docs/COMPONENT-MAP.md are left to the orchestrator; no docs/formats/ page changes.
- [x] fmt, clippy -D warnings and tests for the touched crates on the worker (`make check` scope).

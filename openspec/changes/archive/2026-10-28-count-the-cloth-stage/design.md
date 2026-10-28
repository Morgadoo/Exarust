## Decisions

- **Counters are flag-gated.** With `EXA_DEBUG_CLOTH` unset, the stage takes no timings and keeps no per-cloth list.
- **Once a second.** At 83 ticks a second, a log line every tick would flood the log. Instead the counters log the per-tick means over 83 ticks and the last tick's work per cloth.
- **What they separate:**
  - cloths that never sleep (countdown above 0, awake about 14);
  - body cost (parts offered, swept spheres, swept × cloth-triangle tests);
  - level cost (static records, own-node skips, triangles, particle-triangle tests).

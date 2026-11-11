## 1. Changes

- [x] 1.1 `Ragdoll::list_place` with its insert hook, `list_parts`, `walk_order`; tests `bodies_are_walked_from_the_last_to_join`
- [x] 1.2 `step_bodies`, `rebuild_sector_bodies` in walk order; body grid parts root last
- [x] 1.3 Pair stage: walk order, cell-list meet order, key ties by list place (test `the_earlier_listed_part_brings_its_spheres_on_a_tie`), all-pairs from the list's start (test `the_all_pairs_sector_meets_only_later_parts`)
- [x] 1.4 Bodies built and props registered in spawn order
- [x] 1.5 `the_dropped_weapon_meets_another_body` follows the tie rule (the sleeper joined first and brings its spheres; the weapon ends one bit off in x and z)

## 2. Checks

- [x] 2.1 Duel with `--weapon ub05ssworda`, 1500 frames: 2, 2 and 4 scenes identical (b09563b6... after the main merge, same values as df7d01f3..., body ids one lower); r155 0000370a... x2; first difference tick 325 (the first clash: the tie now goes to the player's weapon, listed first)
- [x] 2.2 Plain duel: 2 and 4 scenes identical (61a25424..., same values as 7c463ebd...); r155 873e2c63... x2; first difference tick 334 (the order the opponent's weapon meets the player's parts)
- [x] 2.3 exanima01, 4 scenes: values identical over the common ticks (lengths 970-1000 ticks vary with the load frames, as r155's do); first difference from r155 tick 1, two placed corpses overlapping props at registration (`--pair-trace`: their parts meet the prop in reverse list order), gaps 2.0 and 0.2 cm, constant
- [x] 2.4 exa_physics and exa_game READMEs, docs/formats/RSQ.md; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
- [x] 2.5 fmt, clippy -D warnings, workspace tests on the worker

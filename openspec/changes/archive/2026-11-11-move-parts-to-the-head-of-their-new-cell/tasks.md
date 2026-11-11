## 1. Research

- [x] 1.1 REA and `exe.asm`: the grid step's migration (awake list from its start, after the repair, head-of-cell link, sleepers not walked), the key word's 16-bit index, the per-part step order and what a part's step reads
- [x] 1.2 REA and `exe.asm`: every routine writing a body's particle sets (the Verlet step's velocity from the previous less the third set; placements and the freeze writing all three)

## 2. Port

- [x] 2.1 `CellEntry` per part and the pair stage's `link_cells` (join in list order, migrate in awake-list order), the cell meeting order by link time
- [x] 2.2 `Ragdoll::set_velocity` writes the two-ticks-ago particles
- [x] 2.3 Tests: the head move, sleepers kept, bodies without a place; a velocity read by the two-step point

## 3. Check

- [x] 3.1 GPU grid, lockstep, 2 runs each against main: the sword duel, the plain duel, exanima01, exanima01 with its props shoved
- [x] 3.2 fmt, clippy -D warnings, tests, play build, `openspec validate --all --strict`

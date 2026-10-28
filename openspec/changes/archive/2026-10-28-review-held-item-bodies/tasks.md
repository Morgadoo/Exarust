## 1. Research

- [x] 1.1 Read the grip builder, the world welds, the mode setter and its callback, the relax pass, the flip step, the held-item refresh, the empty-hand reset, the link init and the part's impact callback as instructions

## 2. Implementation

- [x] 2.1 `exa_physics::grip_solve` cross-body stage, build, mark, release and sync
- [x] 2.2 `exa_physics::grip` builder, mode, relax, flip and weld corrections
- [x] 2.3 `exa_combat::impact` weapon-body branch, its cooldown gate and the stab's stall
- [x] 2.4 `exa_combat::grip` held reach and aim height in double
- [x] 2.5 `exa_character` and `exa_game::torch` give rigid-root held items a body

## 3. Review fixes

- [x] 3.1 Store the requested grip mode, not the callback's turn
- [x] 3.2 Share the limp law with the grip stage
- [x] 3.3 Double multiplies in the held reach and aim height
- [x] 3.4 Gate the weapon sound on the item's cooldown; zero a stab's stall and set its end
- [x] 3.5 Remove retail addresses and object offsets from the notes

## 4. Verification

- [x] 4.1 Remote `test --workspace` and clippy with `-D warnings`

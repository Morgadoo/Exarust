## 1. Research

- [x] 1.1 Re-read in REA and `exe.asm` the dynamic pair solve (slop, reversed friction, drop), the pair pass (shares, edge restore after work), the pair friction (rule, gate), the per-hit push (weights, φ, f, shares, edge-restore accumulator), the manifold, the group-mask builder, the layer test, the order key at add-to-world, the layer setter of legs and feet, the part-mass setter, and every write of the foot's out-of-pairs state bit; constants read from the PE; findings appended to `context/re/body-pairs.md` and the master plan's C17 note

## 2. The pair stage

- [x] 2.1 `Part::pairing` (group and layer masks, out, contact mobility, the sphere set's bounding sphere); `Ragdoll::new` builds the group masks from the tree and registers the legs (lower legs 0x300, feet 0x30, out, the feet's union); `go_limp` sets the lower legs to 0
- [x] 2.2 `Pairs` (`SolverStage`): detection over the awake parts and the sleeping parts in their 3 × 3 cells, each pair once; group test within a body tree, layer test between bodies; the order key; sphere against sphere, static or swept; the immediate push; the four-point manifold (shared with the level's, `contact::evicted`); touches kept; sleep countdowns equalised
- [x] 2.3 The pair pass at the start of each iteration (records mapped to the bodies by key at the first), dropped records, records cleared after the iterations; the pair friction at the start of each friction pass
- [x] 2.4 `SolverStage::detect` and `friction` see the sleeping bodies (`solve_among`); `step_bodies` passes them without marking them changed unless woken; `BodyPlugin` registers `Pairs`
- [x] 2.5 Debug helpers: `exarust --pair-trace <file.csv>` (`exa_game::pair_trace`) and `exarust --corpse x,y,z`
- [x] 2.6 On M32 and L5: `Part::rigid` gets its pairing (its mobility) and `Ragdoll::rigid` the group masks of its tree; no body meets a stand-in (the meets-movable rule gone); the collapse's start sets the lower legs' layer to 0

## 3. Tests

- [x] 3.1 Two limbs thrown at each other stop at their spheres (0.6 cm in the tick they meet, ninth tick) and part; without the stage they pass through
- [x] 3.2 The human self-collision matrix from the group masks (13 bits; the feet's union)
- [x] 3.3 Layer channels (0x300/0x300, 0x300/0, 0x30/0x10, 0x10/0x10, 0x40/0xC0, bits 0–3); living lower legs pass through, dead ones meet, feet never
- [x] 3.4 Order keys 333 (pelvis), 60 (hand), 2 (half to even), 0 (frozen)
- [x] 3.5 The immediate push: φ 1 and 4, f = 0.4, 1.6 and 0.4 shares, momentum unchanged
- [x] 3.6 The pair pass: 0.5 cm taken back by the shares; reversed friction (0.2 cm loses 0.000625, 0.3 cm cancelled; n_y 1: 2.4 cm kept, 2.6 cm cancelled); within the slop nothing moves
- [x] 3.7 The pair friction: 0.2 cm cancelled, 0.3 cm loses 0.000625 of itself; under the 1.19e-7 gate (1.1e-7) a 3e-4 cm slip is kept, at 1.2e-7 cancelled
- [x] 3.8 A pair dropped once satisfied stays dropped for the tick
- [x] 3.9 A falling body wakes a sleeper with its countdown (254) and clears its level contacts; two sleepers never meet
- [x] 3.10 A light body dropped on a heavy sleeping one rests on its spheres (14 cm from the nearest centre) and both sleep again
- [x] 3.11 The four-point manifold; the plugin registers the stage; the pair trace's row
- [x] 3.12 A cup dropped on a sleeping table body wakes it and rests on it; a limb thrown at a prop pushes it; a prop and a limb fall through a stand-in; a registered prop's pairing; a child skips its parent; the collapse's start drops the living lower legs' layer

## 4. Verification

- [x] 4.1 Real installation (skipped without `EXANIMA_DIR`; no data format changes, so no `exa-tool` corpus command applies): light runs through `gpu_run.sh` on the RTX 4000: `--opponent --pair-trace play arenao1` (six hits land on the player, as before; the only contacts are a body's own: the duellist's hand on its thigh under 1.5 mm, the dead player's arms on its trunk, at most 1.05 cm); `--corpse 0,0,3.4 --press W@150-420 --pair-trace --joint-trace play arena` (the player's lower legs meet the corpse's thighs, chest and arms for 45 ticks, pushes up to 1.6 cm; the corpse, counting down at 20, wakes to 256 and sleeps 130 ticks after the last contact); exanima01 did not fit in the free GPU memory
- [x] 4.2 Remove probes and dumps from the tracked tree
- [x] 4.3 On main with M32 and L5: `--at 11.2,4.5,12.8 --shove-props 120 play arenao1` (8 shoved, six back on the table 45–51 cm away, two on the floor, all asleep after 324 ticks, one prop–prop contact); `--corpse 0,0,3.4 --press W@150-420` (48 ticks of contacts, pushes ≤ 1.75 cm); `--opponent` (five hits, the collapse, Rebalance, death 250 ticks after it)

## 5. Documentation

- [x] 5.1 `docs/formats/RSQ.md` (Physical body: contacts between bodies, sleep), `docs/DEBUGGING.md` (`--pair-trace`, `--corpse`, the pair trace's columns), `docs/PLAN.md` (Phase 3, next steps), READMEs of `exa_physics`, `exa_game`, `exarust`
- [x] 5.2 `openspec validate --all --strict`
- [x] 5.3 `make check` (fmt, clippy -D warnings, tests; `play` profile)

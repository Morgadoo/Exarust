## 1. Joints

- [x] 1.1 `exa_physics::body`: `PartSetup` gains `origin` (the part's origin in its parent) and `foot`; `Ragdoll::new` places each stored parent point at the stored point plus the origin and appends the origin pivot (slack 0, strength 1) to every part with a parent
- [x] 1.2 Shares m/(m + m_parent) with m = 0.175 × the bone's mobility; corrections spread by the points' weights, unclamped (`Part::push`); the generalised-inverse-mass split removed
- [x] 1.3 `Link::living` (pivot d/(1 + 0.001|d|) in full, with the 80 cm report; slack excess at full weight, cut to 80 cm) and `Link::limp` (pivot strength/(1 + 0.001|d|²), slack excess × strength)
- [x] 1.4 Orders from the part tree: children first for living bodies, parents first depth first for limp ones (`tree_orders`); each part's links in one range (`links_of`)
- [x] 1.5 Strength (leg limit 0.2 on a lower leg with foot contacts, the femur's limit index) and setup smoothing per link; `Ragdoll::go_limp` restores the leg limit strengths
- [x] 1.6 `exa_game::limp`: `part_setup` passes the stored points, the model's translation of the part under its parent part and the foot flag; `go_limp` goes through `Ragdoll::go_limp`; `rest_under`'s rotation no longer reaches the links

## 2. Tests

- [x] 2.1 Synthetic: a femur has 3 links and a hand 4; pivots at the origins; parent points unturned by the part's rest rotation; shares 0.588/0.412 and 0.609/0.391; leg limits, smoothing (0.755, 0.914, 0.836, the 0.99 cap, 0 at the origin) and the limp reset; the pivot and limit laws and the 80 cm cap; the tree orders
- [x] 2.2 Synthetic humanoid with its part origins at the joints: the hips stay within 2 mm of their sockets under a 4 m/s torso blow, living (0.21 mm) and limp (0.03 mm); a knee struck sideways and forward stays within 5° of its plane (0.62°, 0.67° living)
- [x] 2.3 Existing tests ported to stored-point setups with origins (the momentum test passes with the new shares)
- [x] 2.4 Corpus (skipped without `EXANIMA_DIR`; `humanoid.rsq` and `humanbase.rfc` through the game's `part_setup`): every part has its stored links plus the pivot, femurs 3, hands 4; every stored parent point is as long as its part's point; the right humerus's first link at (−21.58, −8.56, 6.51) cm, the right hip pivot at (−9.07, −8.46, 3.50) cm and the knee's at (−0.30, −43.19, −1.09) cm; both lower legs' limit links at 0.2

## 3. Verification

- [x] 3.1 In game through `gpu_run.sh`, with a temporary joint-gap probe (removed), against the build before this change: R6 `--limp character` (joint gaps at rest mean 1.9 mm, max 6 mm, was 81 mm and 150 mm; asleep by tick 664; ends kneeling, forehead on the floor); R5 `play exanima01` (7 corpses: mean gap 4.3 mm, was 92 mm; one corpse standing on a cot keeps a hip 8 cm out with its thigh under the cot; 3 of 7 asleep after 50 s, was 2; living bodies 3.5 mm, was 16–33 mm and growing); R3 `--shove 60 play arena` (falls at tick 293, gets up with `Get Up 2` at 516, idle at 662; standing gap 0 mm, was 5.3 mm; fallen ≤ 24 mm, was 170 mm); `character Default/Walk` (≤ 4.3 mm at the feet)
- [x] 3.2 Remove probes and dumps from the tracked tree

## 4. Documentation

- [x] 4.1 `docs/formats/RSQ.md` (Physical body: where the parent points lie, the origin pivots, shares, the living and limp laws, strength and smoothing), `docs/PLAN.md` (Phase 3), `crates/exa_physics/README.md`
- [x] 4.2 `openspec validate --all --strict`
- [x] 4.3 `make check` (fmt, clippy -D warnings, tests; `play` profile)

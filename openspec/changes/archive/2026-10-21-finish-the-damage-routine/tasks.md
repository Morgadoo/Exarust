# Tasks

## 1. The routine

- [x] 1.1 `resolve` in the routine's order and precision, with the zone paths, the haft's material crush bonus at any on-target share, the fast power and the daze's stall factor; tests pinned to a model of the listing (bare and plated cut, slow overhead, stab, mace haft and head, punch, claws, Security, steadfast, a shield, past the tip).
- [x] 1.2 The outputs: impact at the hit, speed, total, zone, edge, body and haft flags, smoothing, blow sag, hit sounds and layer ringing (`hit_sounds`).
- [x] 1.3 `fast_math`: the fast logarithm and exponential, with their edges.
- [x] 1.4 A quadruped's stab resolves as a swing (`CharacterStats::quadruped`, test).

## 2. The hit

- [x] 2.1 The hit reaction feeds the victim's rules, draws no roll for a guarded blow and the part sound's number for a blow on flesh, uses the blow's stall factor, and sends the blow sag with the shove.
- [x] 2.2 The striker after a wound and its recoil (`SwingState::after_wound`, `recoil_from`), wired and tested; a body that turns blows takes nothing.
- [x] 2.3 The blow sag written on the struck body (`Ragdoll::sag_from_blow`, test).

## 3. The holding pass

- [x] 3.1 The holding pass in double where the original multiplies in double, its non-number branches; bit-exact test.

## 4. Docs and check

- [x] 4.1 Update `docs/formats/OTHER.md` (Damage, Weapon record), `docs/formats/RSQ.md` (Pose hold) and the `exa_combat` README. `docs/PLAN.md` is the integrator's.
- [x] 4.2 Corpus: no format change; the record's corpus tests still pass. Remote tests and clippy for `exa_combat`, `exa_formats`, `exa_physics`, `exa_character`, `exa_game`, `exarust`, and `openspec validate --all --strict`, pass.

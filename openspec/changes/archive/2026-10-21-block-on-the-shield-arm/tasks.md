# Tasks

## 1. The block

- [x] 1.1 `damage::block` and `damage::parried` with the shield parry, Synchrony, Persist, Counter and potion rules; bit-exact tests from a model of the listing.
- [x] 1.2 `Guard::Shield(ShieldGuard)`; `guard_outcome` removed.

## 2. The hit

- [x] 2.1 The hit reaction applies the block (stamina kept, yellow leak, daze and stall factor, sag, lock, low-health check) and the weapon parry (attack state cleared, lock without Riposte, sag), and recoils the striker; tests on both.

## 3. Docs and check

- [x] 3.1 Update `docs/formats/OTHER.md` (Damage) and the `exa_combat` README. `docs/PLAN.md` is the integrator's.
- [x] 3.2 Corpus: no format change. Remote tests and clippy for `exa_combat` and its dependents, and `openspec validate --all --strict`, pass.

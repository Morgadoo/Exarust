# Tasks

## 1. Fixes

- [x] 1.1 Round the recoil's and the follow-through's end half to even; tests pin 0.52 ms and 0.54 ms rounding up.
- [x] 1.2 A stab's pierce takes the root branch when its energy is no number; test on a stab with no contact number.
- [x] 1.3 A turned blow sends its blow sag; test.

## 2. Docs and check

- [x] 2.1 Update `docs/formats/OTHER.md` and the `exa_combat` README. `docs/PLAN.md` is the integrator's.
- [x] 2.2 Corpus: no format change. Remote tests and clippy for `exa_combat` and its dependents, and `openspec validate --all --strict`, pass.

# Tasks

- [x] Port the combat state's parts (practice, enter, estimate, weapon draw, lunge, chase and eagerness, drives, footwork, place search, attack decision, friend test, keep-off, step, guard and turn) as `exa_ai::combat`, with tests pinned to the routine's numbers.
- [x] Compare the attack choice's reach, scale, crouch and energy in double precision, order its facing score, and take the executor's answer before the swing turn draw.
- [x] Run the duellist on the combat state once per body tick, with the position score as its field.
- [x] Corpus: no format field changes, so `cargo exa` is skipped.
- [x] Update `crates/exa_ai/README.md`; research notes in `context/re/npc-combat-state.md`. `docs/PLAN.md` and `docs/COMPONENT-MAP.md` are left to the integrator.
- [x] Remote tests and clippy `-D warnings` for `exa_ai` and `exarust`.

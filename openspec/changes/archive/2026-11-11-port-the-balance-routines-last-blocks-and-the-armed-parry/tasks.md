## 1. Changes

- [x] 1.1 Trigger: every kind re-targeted, the 3-D lead, the landing count reset, the writes while locomoting, no start clock for a balance step; tests `a_knockback_step_is_re_targeted_like_any_step`, `a_locomoting_trigger_writes_its_target_but_starts_no_step`, `a_balance_step_writes_no_start_clock`, `the_target_leads_the_chest`
- [x] 1.2 Footwork: slow-turn hold, other foot's clear, queued step on a changed tick; tests `the_slow_turn_holds_a_reached_foot`, `the_other_foot_lands_near_the_target`, `a_queued_step_is_taken_on_a_tick_a_foot_changed`
- [x] 1.3 Scores: the pelvis eased toward the stance origin, the uncrouched reach
- [x] 1.4 Armed parry links; tests `an_armed_parry_aims_its_slots_parry_link`, `the_parry_drives_the_soft_weld_not_the_grip`, the parry wiring and AI parry-end tests on the soft weld
- [x] 1.5 Stab handling pushes applied to the item body and the right hand; the duplicate drive removed

## 2. Checks

- [x] 2.1 exanima01, plain duel, sword duel and a combat stepping scene lockstep, 2 runs each, against kernel3 and a build without the reach reference (context/artifacts/2026-10-30-balance)
- [x] 2.2 exa_physics and exa_combat READMEs; docs/re/motiledyn-map.md; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
- [x] 2.3 fmt, clippy -D warnings and tests of the touched crates on the worker

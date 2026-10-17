## 1. Regeneration

- [x] 1.1 `Health` gains `rate` and `heal_pool` (0 at spawn, `Default`); `Health::take` sets the rate to 0 first
- [x] 1.2 `exa_combat::regen`: `RATE_CAP` (0.01/83 in float), `RAMP` (0.0003, double), `RED_PER_TICK` (0.25 × 5e-5, double), `RED_FLOOR` (1.19209e-7); `Health::regenerate_red(rules)` (+1.25e-5 up to the maximum when `regenerates_red`); `Health::regenerate(held)` (heal pool → red, the rate held at 0 or ramped to the cap, yellow += √red × rate above the floor, yellow within red, red within the maximum, yellow at least 0); `holds_regeneration(condition, motion)` (condition 2 or action 4)
- [x] 1.3 `HealthTick` (after `PostureTick`, before `CharacterTicks`) and `regenerate_health`: red regeneration for every fighter, then, unless dead, the rest with the gate from the condition and the clip playing (its action index in the installed motion set); the component written only when it changed; a debug line when the rate reaches its cap; `RegenerationPlugin` in `CombatPlugin`
- [x] 1.4 `react_to_hits`: every hit on a fighter sets its rate to 0 before it is resolved; `dying_tick`: death sets yellow to 0
- [x] 1.5 `exa_formats::character_stats::BodyRules::regenerates_red` (0x20 without 0x100)

## 2. Tests

- [x] 2.1 Unit (`regen_test.rs`): `the_rate_reaches_its_cap_in_40_seconds` (1.2048193e-4; 3.6144577e-8 after a tick; below the cap at 3,333, at it on 3,334), `the_retail_trace_fits` (0.08641, 0.09018, 0.09974, 0.10998, 0.12093, 0.20089 at 5.0–45.8 s, each within 0.0012 of the trace's 0.086–0.202; red 0.203 the ceiling), `a_human_heals_0_005_a_second_at_the_cap` (0.004999996 a second; 0.19963 after 40 s from 0.1), `held_it_does_not_heal`, `attacking_or_the_combat_idle_hold_it`, `a_blow_restarts_the_ramp`, `the_clamps` (yellow follows red a tick later; floor 1.19209e-7), `red_regenerates_by_the_rule_bits` (0.10104 after 83 ticks, 0.1125 after 1,000; 0x170 and 0x12 none; capped at 1.5), `the_heal_pool_drains_into_red` (0.1000125/0.1999875 after a tick; 15,999 ticks; red full after 11,999), `the_health_tick` (heals in the Defence stance, held in Cmbt - Idle and attacking, the dead regain red only)
- [x] 2.2 Unit: `collapse::the_body_dies_250_ticks_after_the_blow` with the health tick (yellow 0.0005625 after 249 ticks, 0 at death and after); `character_stats::body_rule_bits` (`regenerates_red` for 0x67 and 0x70, not 0x170, 0x140, 0x12)
- [x] 2.3 Corpus (skipped without `EXANIMA_DIR`): `shipped_level_characters` (red regenerates for Sir and the three harvesters, race bytes 4, 10, 10, 10), `shipped_arena_actors` (only the demon)

## 3. Verification

- [x] 3.1 `exe.asm` and the PE: the health bookkeeping (heal pool, the gate on condition 2 and motion 4, the ramp in double, the cap float with three readers and no writer, √red × rate in float above 1.19209e-7, the clamps' order, yellow ≥ 0 at the tick's end), the prologue's red regeneration (& 0x120 = 0x20, before the dead test, 0.25 float × 5e-5 double), the rate resets in the damage call and in the damage routine (the victim's, before the contact's branches), death's yellow 0; findings in `context/re/master-body.md` §22
- [x] 3.2 In game (light runs on the RTX 4000, `--no-gi --no-supersample --size 960x540`): `--still-opponent --at 0,0,1 --health 0.085,0.203 --press Tab@50 play arenao1`: yellow held in Cmbt - Idle, then 0.086 at 4.4 s, 0.101 at 16.9 s, 0.118 at 24.4 s, 0.172 at 39.4 s after the Tab (the rule within the log's three decimals), the full rate 40.16 s after it, the bars shown out of combat mode with the yellow refilling over the brown (shots at frames 300 and 2500); `--opponent --press Tab@200,S@210-2000 play arenao1`: after the first blow (tick 376, yellow 0.213) yellow rose to 0.221 in 11.1 s, each later blow restarted the ramp, the collapse and death 250 ticks after the killing blow left yellow 0

## 4. Documentation

- [x] 4.1 `docs/formats/OTHER.md` (Regeneration), `docs/formats/RFC.md` (rule bit 0x20), `docs/formats/RSQ.md` (Condition: the health tick reads it), `docs/UI.md` (the bars while healing), `docs/PLAYER.md` (Healing), `docs/DEBUGGING.md` (the regeneration log and a healing shot), `docs/PLAN.md` (Phase 4), `crates/exa_combat/README.md`
- [x] 4.2 `openspec validate --all --strict`
- [x] 4.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`

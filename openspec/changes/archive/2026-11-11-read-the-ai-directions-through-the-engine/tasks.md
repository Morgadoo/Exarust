## 1. Changes

- [x] 1.1 Engine routines in `motion` and `search`; `model_yaw` for the model; tests `yaw_and_direction_agree`, the head look, the look chain, the sweep
- [x] 1.2 Snap copies the facing vector; settle reads the heading direction; tests `the_attach_snap_takes_the_body_facing_once`, `the_aim_settle_and_snap_take_the_body_yaw`
- [x] 1.3 `Npc::facing` and `Npc::turn_model` in the idle turn, flight, roam and companion walks; test `an_idle_brain_keeps_its_placed_facing` (200 ticks; 4.5e-3 rad of drift without the remembered yaw)
- [x] 1.4 Revive reset in the fixed step's world, footprint lift and stamp, room per round; test `a_roaming_reset_lifts_and_stamps_its_footprint`

## 2. Checks

- [x] 2.1 Plain and sword duels identical to r158 (2 runs each identical)
- [x] 2.2 exanima01: 2 runs identical over the common ticks; first difference from r158 tick 2, the four standing NPCs the stand-in turns (yaw within the heading's error)
- [x] 2.3 Story revive on exanima08 and exanima04: 48 of 49 and 8 of 9 roaming brains moved (none before)
- [x] 2.4 exa_ai and exa_game READMEs, docs/NAVIGATION.md; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
- [x] 2.5 fmt, clippy -D warnings, exa_ai and exa_game tests on the worker

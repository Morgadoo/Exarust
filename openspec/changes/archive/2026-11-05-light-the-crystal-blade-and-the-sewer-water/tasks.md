# Tasks

- [x] Trace the item ability's install, tick, type checks, lighting, putting out, hit hook, the socket drop and the player light's rule in REA.
- [x] Find which records carry an ability and how its state is set: `objdb.rdb` and every level's local database.
- [x] Trace the scene's water level: environment frame, terrain load, save reset, both wet steps.
- [x] `exa_game::item_ability`: tick, flame row, light, loop, ribbon draw, sizzle, overlay, `--flame-sword`; tests.
- [x] `exa_character::held::HeldModifier`; player light's lit-blade rule.
- [x] `exa_physics`: `WaterLevel`, `water_level`, `gait_wet_step`, contact on the cues; tests.
- [x] `exa_world`: terrain wetness into the environment, `water_level` frame value published; tests renamed.
- [x] `exa_audio::footsteps`: wet steps on both paths.
- [x] Crate READMEs.
- [x] GPU check in the grid: `play exanima01 --flame-sword`, `play exanima05_sw --autowalk`.
- [x] fmt, clippy -D warnings and tests for the touched crates on the worker.

## 1. Changes

- [x] 1.1 Core test by race byte + 1; test `shipped_golems_take_their_own_cores`
- [x] 1.2 `Race::wears_apparel`, `RaceStats::mind`, `CharacterStats::mind`; test `shipped_races_dress_and_have_minds`
- [x] 1.3 Dressing by the look bit in `scene_bodies` and the play-mode placement
- [x] 1.4 `attach_minds` reads the race's mind word; cast log
- [x] 1.5 Underwear rule by the body's race; tests `put_on_underwear_beneath_a_bare_outfit`, `keep_underwear_only_marks`
- [x] 1.6 `exa-tool races` prints look and mind words

## 2. Checks

- [x] 2.1 Grid: Calm on spawned skeleton, ghoul, golem guard and ogre against r195
- [x] 2.2 Grid: the exanima05_sw manipulators dressed (after) and bare (r195)
- [x] 2.3 Lockstep traces against r195 and the determinism grid
- [x] 2.4 fmt, clippy -D warnings, tests on the worker; crate READMEs

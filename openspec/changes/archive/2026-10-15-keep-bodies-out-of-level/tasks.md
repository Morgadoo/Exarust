## 1. Level contacts

- [x] 1.1 `contact.rs`: the level as triangle meshes (rapier trimeshes in the game, triangle lists in tests), the one-sided sphere test, the swept test, the contact record and the friction law
- [x] 1.2 Tick order: links, stored contacts and edges in each of the 20 iterations; detection once per tick with an immediate push and up to four stored contacts per part; 8 friction passes; the last tick's pose kept per part
- [x] 1.3 Sleep and carry: a sleeping body is not tested; waking clears its contacts; the controller carry moves the last pose and the contacts
- [x] 1.4 Mirrored level meshes keep their front side

## 2. Drive

- [x] 2.1 Per-tick target steps turn each part about itself
- [x] 2.2 `fall()` clears the aimed pose; a fallen body's muscles act inside the body (limbs damped relative to its rigid motion, bulk velocity and spin; no net shift or turn of the body)
- [x] 2.3 Links split each correction by the points' generalised inverse masses (part inverse mass × squared weights), so the constraints conserve momentum

## 3. Tests

- [x] 3.1 A shoved body far from the origin does not sink through a floor; a sphere crossing a floor within a tick is taken back; a centre behind a face is not pushed; a fast ball does not tunnel; a thin wall stops balls from its front only; a limb dragged by its links stays above the floor; a limp body rests, does not creep and sleeps; friction sticks and slides; the four-contact limit; the carry
- [x] 3.2 Corpus check over every level: share of near-horizontal level triangles facing up, mirrored nodes (skipped without EXANIMA_DIR)
- [x] 3.3 In the game: `exarust play exanima01 --shove 120` — no pelvis ends below the floor and no body drifts away; limp bodies and placed corpses rest on the floor

## 4. Docs and checks

- [x] 4.1 `docs/formats/RSQ.md` (Physical body), `crates/exa_physics/README.md`, `docs/PLAN.md` Phase 3
- [x] 4.2 `openspec validate --all --strict`
- [x] 4.3 `make check`

## 1. Find
- [x] 1.1 REA: the level change's portal bit, the portal use, its system build, the seeding and its sound, against the assembly
- [x] 1.2 The sparks and dissolve templates, their keys and the `_PrtlPrt01` record
- [x] 1.3 The marker import and the portal effect's spawn (the ring head)

## 2. Wire
- [x] 2.1 The emitter's portal ring spawn and seeding pass
- [x] 2.2 Any-texture and additive flames; the additive material
- [x] 2.3 The marker's spark system with its light
- [x] 2.4 `PortalUse`: the dissolve over the body's parts and `portaluse01`
- [x] 2.5 `Step::PortalUse`'s executor; `--debug-portal-use`, `--debug-exit`
- [x] 2.6 Tests (the ring spawn and its full checks, the seed pass, the sparks node, the templates, a portal use)

## 3. Check
- [x] 3.1 fmt, clippy, tests
- [x] 3.2 GPU: the sparks in exanima03/04 and the burst (arena, and the real exit in exanima03/04), before and after (`context/artifacts/2026-10-30-portal`)
- [x] 3.3 The plain arena duel and exanima01 lockstep traces identical to r155 on every common tick (the body id column aside)

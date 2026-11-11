## 1. Find
- [x] 1.1 REA: `CreateFire`'s texture per tier against the assembly
- [x] 1.2 REA: the shell creator's callers (the vanish start, the constant start, the summon), the constant's control end, the vanish's reuse and destroy
- [x] 1.3 REA: the segment pass's fade half, the per-node show call, the stage call's fade bit, the cube caster pass's dithered branch

## 2. Wire
- [x] 2.1 The fire texture selection
- [x] 2.2 `ConstantShell`: made by the constant's cast, stepped, closed at the power's end, drawn
- [x] 2.3 The vanish and Long Door's shared flag and their lend of the constant's shell
- [x] 2.4 The fade half's blended body
- [x] 2.5 `ShadowDither` in the cube shadows and the vanish's dither
- [x] 2.6 Tests (the texture pick, the constant's shell, the lent shell, the blended body and the dither)

## 3. Check
- [x] 3.1 fmt, clippy, tests
- [x] 3.2 GPU: torches and braziers before and after; the constant's shell, a vanish over it, the fade half and the hidden body's shadow (`context/artifacts/2026-10-30-shell`)
- [x] 3.3 The plain arena duel and exanima01 lockstep traces identical to the base on every common tick

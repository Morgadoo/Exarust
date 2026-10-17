## 1. The grip table

- [x] 1.1 Confirm in REA/`exe.asm` and the installed executable's bytes: the hand-class routine (second slot → 5, high nibble 9 → 1, bits 4–6), the six entries (hand, offset, ogre offset, angles, links, two-hand and stab links, callbacks), the placement (ogre race byte 3 copied unscaled, × size otherwise, race bytes × 0.1 cm with x by hand), the rotation builder's rows and the flip (rows X and Z negated); record the findings and corrections in the research notes
- [x] 1.2 `exa_combat::grip`: `GRIPS`, `Grip` (hand, offset, ogre offset, Euler angles, links, second hand, stab, modes), `hand_class`, `grip`, `GripHolder`, `Grip::origin`, `Grip::rotation` (flip), `Grip::placement`

## 2. Holding items by it

- [x] 2.1 `exa_character`: `RaceWeaponry` carries the race's held offset (`of_race`), `grip_holder(size)`, `grip_bone`; a wielded weapon spawns under its grip's hand with its grip's placement for the fighter's size (1 without a fighter) and race, replacing `grip_transform`
- [x] 2.2 `exa_game::torch`: the torch's grips from the table (the one-handed grip as the first held item, the off-hand grip as the second) for the holder's size and race; its own `Grip`, `MAIN_GRIP`, `OFF_GRIP` and `LEFT_HAND` removed

## 3. Tests

- [x] 3.1 `grip`: the hand class (fists 0, sword 0x11 and torch 0x13 → 1, greatsword 0x521 → 2, polearm 0x34 → 3, claws 0xC0 → 4, two weapons 0x90 → 1, 0x50 → 5, 0x60/0x70 none, the second slot 5); every entry's numbers; the origin for size 1 (−12, −1.5, 6) cm, 1.05 (−12.6, −1.575, 6.3), the ogre (−27.5, −7, 9) and (30, −4, 14) at sizes 1.5 and 2.2, the demon (−27, −2.25, 9) and (25.5, −0.75, 9), a y/z race offset; the rows of every entry and the flipped rows; the one-handed blade along (−0.3562, 0.3214, 0.8774)
- [x] 3.2 Corpus (skipped without `EXANIMA_DIR`): the 208 weapon items of `objdb.rdb` take grips 1/2/3 136/40/32 times, the torch grip 1; `races.rdb`: the ogre's race byte is 3, only the demon (byte 11) has a held offset (9 cm along x); the factory "Sword 2hA", "Halberd A" (left hand) and "Falchion A" take grips 2, 3 and 1
- [x] 3.3 `torch`: each hand's grip is the table's, and a human of average height holds it exactly where the hand-copied grip did

## 4. Verification

- [x] 4.1 Light character-view runs through `gpu_run.sh --gpu 4000` (`--no-gi --no-supersample --size 960x540`), before and after: "One Handed" Cmbt - Idle with `ub05ssworda` (front, side, from above), "Greatsword" Cmbt - Idle with "sword gsf 01" against the live two-handed capture, "Polearm" Cmbt - Idle with "@Halberd A1", "Torch" Idle and Cmbt - Idle (body, arm and torch pixel-identical; only the flame's flicker on the floor differs)
- [x] 4.2 Remove probes and scratch files from the tracked tree (none were added)

## 5. Documentation

- [x] 5.1 `docs/PLAYER.md` (Held items and their grips; The combat stance; The carried torch), `docs/PLAN.md` (Phase 4), crate READMEs (`exa_combat`, `exa_character`, `exa_game`); no `docs/formats/` page changes (the race's held offset is already in OTHER.md)
- [x] 5.2 `openspec validate --all --strict`
- [x] 5.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`

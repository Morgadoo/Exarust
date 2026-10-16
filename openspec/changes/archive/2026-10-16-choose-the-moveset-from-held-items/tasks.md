## 1. The rule

- [x] 1.1 `exa_character::held`: `Hands` (main item flags and light, second weapon, shield style, off-hand light), `StanceClass`, `HeldMoveset` (moveset, stance class, weapon class) and `choose(hands, base, claws)` as the original's rule: shield, off-hand light, main-hand light, then the hands bits (one-handed or two weapons, two-handed by 0x521 and kind bit 0, polearm), else the race's moveset with 0x80 or 0xC0; weapon classes masked to the high nibble
- [x] 1.2 A system after `equip_outfits` recomputes `HeldMoveset` when `Hands` changes and writes its moveset into the body's `Animator`; `Hands` requires `HeldMoveset` (a human's empty hands)
- [x] 1.3 `equip_outfits` puts the wielded weapon's item flags in the main hand; lighting the carried torch puts it in its hand (main as a light with the torch item's flags 0x13, or the off-hand light)
- [x] 1.4 `PlayerBody` requires `Hands`; the duellist is spawned with `Hands` and the "Default" moveset until dressed

## 2. The player

- [x] 2.1 Exploration steering and Tab change only the action, never the moveset; the player's get-up ends in its stance by its combat flag (`balance`)
- [x] 2.2 Combat facing: the heading turns to the cursor once it is more than 30° off and follows it while the left button is held; the body turns toward it with `body_turn` each tick; the footwork steps along the cursor's direction (`aim`, reset to the heading by Tab)
- [x] 2.3 Attack choice: a click within 300 ms of the last is an overhead (and clears it); otherwise the other side when the cursor is more than 20° left of the body's facing (15° for weapon class 0); each side's clip picked by the original's generator, redrawn while equal to the side's last pick (0 at first); an overhead not over a Stab or Overhead; the sides keep the existing gate (no attack running, a looping clip); the log names the attack, moveset and cursor angle

## 3. Debug helpers

- [x] 3.1 `--press` accepts `LMB`, `RMB` and `MMB`, pressed and released like keys
- [x] 3.2 `--cursor x,y` holds the cursor at a window point in play mode (`ScriptedCursor`)

## 4. Tests

- [x] 4.1 `held`: empty hands (Default, 5, 0x80; a race's own moveset; claws 0xC0; items without hands bits), weapons (One Handed 0x10 with haft bits above 16, 2W Light 0x90, Greatsword 0x521, 2H Light for kinds 1 and 3, Two Handed for kind 2, Polearm 0x30), shields and lights (1H & Shield / C1 / C2, 1H & Torch with a sword and with empty hands at class 0, Torch at class 0x10), and the system switching a body's clips (One Handed → Default on the first update, Default → One Handed when a sword's flags 0x11 fill the main hand)
- [x] 4.2 `player`: the player holds its fists (Default, 0x80); the side threshold at 19.5°/20.5° (and 14.5°/15.5° for class 0), left only, across the ±π wrap; the double click at 290 ms and the clicks after it (400 ms, 750 ms, 1.1 s); 200 picks never repeat, the first is never clip 0, each clip 50–84 times
- [x] 4.3 `torch`: the held torch picks the "Torch" and "1H & Torch" movesets its hand names
- [x] 4.4 `exarust`: `--press` reads `LMB`, `RMB` and `MMB` and refuses unknown buttons (`lmb`, `XMB`)
- [x] 4.5 Corpus (skipped without `EXANIMA_DIR`): the 208 weapon items of the installed `objdb.rdb` pick "One Handed" 136 times, "Polearm" 32, "Two Handed" 19, "2H Light" 18 and "Greatsword" 3, each resolving combat idle, Defence stance, Attack Mid and Idle in the installed `humanoid.rsq`; "Torch A01" shows `torcha02` with weapon flags 0x13 in the low 16 bits; the "Default" Attack Mid, Down and Up are one clip, Combo 1, 2 and 3 another

## 5. Verification

- [x] 5.1 In game through `gpu_run.sh --gpu 4000`, no panic or validation error in any log:
  - `character "Default/Attack Mid"` at frames 85 and 120: the punch's wind-up and its guard (the clip loops 20–110)
  - `--opponent --press Tab@100,LMB@200 play arena`: "attack: Attack Up in Default (cursor 0° left of the facing)"; no hit from the fists
  - `--opponent --sim-speed 0.25 --camera 4.2,1.4,0.6,90,-8 --press Tab@60,LMB@100 play arenao1`: the wind-up at frame 252, and at frame 355 the right straight, the right arm out to the duellist's chest, the body twisted into it and the left fist back, as in reference shot 4; the duellist with its sword in "One Handed"
  - the chase camera with `--cursor 650,280 --press Tab@60,LMB@62`: "Attack Combo 3 in Default (cursor 51° left of the facing)", the left arm out at frame 317; with the cursor left but the click 40 frames after Tab, the body had turned to 13–14° and the attack was "Attack Up" (the main side)
  - `LMB@100,LMB@106`: "Attack Up" then "Overhead Swing" on the second click
  - `--weapon ub05ssworda`: idling out of combat with the sword in "One Handed"; "Attack Up in One Handed" landing on the duellist's head; a click out of combat starts nothing
  - `play exanima01`: the torch raised in the right hand ("Torch")
- [x] 5.2 Remove probes and scratch files from the tracked tree (none were added)

## 6. Documentation

- [x] 6.1 `docs/PLAYER.md` (Moveset from held items; Combat mode: facing and attacks), `docs/formats/OTHER.md` (the flags word picks the moveset; corpus counts), `docs/DEBUGGING.md` (`--press` buttons, `--cursor`, an unattended attack), `docs/PLAN.md` (Phase 4), `crates/exa_character/README.md`, `crates/exa_game/README.md`, `crates/exarust/README.md`
- [x] 6.2 `openspec validate --all --strict`
- [x] 6.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR` (285 passed, 3 ignored)

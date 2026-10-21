## 1. Research

- [x] 1.1 Read the behaviour tick's combat branch, the aim cursor and aim point, the bone helper, the contacts, the engaged test, the screen-nearest search, the attack timing, the heading helpers in the pack's assembly; the controls' frame, blocked turn, per-frame request, use routines, timed list, start and end of play and light switch; REA for the wrap and sign helpers and the GUI's mouse events

## 2. Implementation

- [x] 2.1 `steering`: `combat_request`, `combat_walk`; `player::steer_player` calls them in combat mode; `cursor::pick_cursor` shows the chevrons by `PlayerBody::walking_to_cursor`
- [x] 2.2 `player::pad_aim_cursor` in the behaviour tick, `AimCursor` read by `player_combat`
- [x] 2.3 `hand::Holding::Mechanism`, `hand::box_ok`
- [x] 2.4 `idle_orbit::Frame` blocked, modal and window gates; the in-game menu as a window
- [x] 2.5 `player_behaviour`, `player_controls`, `player_light::weapon_puts_out`
- [x] 2.6 Remove the lane's duplicated and invented models; keep the entering-combat cancel and the reach latch as reviewed

## 3. Tests

- [x] 3.1 `exa_game`: the combat walk (walk, run, the sprint latch, within 50 cm, the release); the hand's mechanism hold and box limits; the idle gates; the aim ease (0.046656), lock (1.15 at 15 px), raised point, bone fraction (0.5, 16 px²), contacts (0.9166667, 0.75, 0.25, 0.0625, eviction), engaged enemy, screen-nearest, wind-up (44, 330, 42, 0.01) and recovery (0.8017492, 286); pour (0.5, 0.4), clean (0x20), repair (15, 0, 49, none), timed value (50, 25, 48, 105) and list; start and end of play

## 4. Docs and checks

- [x] 4.1 `docs/PLAYER.md`, `docs/UI.md`, `crates/exa_game/README.md`
- [x] 4.2 `cargo fmt --all`, remote `test` and `clippy -D warnings`; `openspec validate --all --strict`

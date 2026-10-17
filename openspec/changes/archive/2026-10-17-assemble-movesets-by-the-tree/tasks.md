## 1. The tree and the table

- [x] 1.1 `exa_formats::rsq`: `MotionSet::parent_of` (the nearest earlier moveset named exactly as the parent; roots for an empty or unmatched name), `moveset_index` (first by name, ASCII case ignored)
- [x] 1.2 `MovesetConfig` (the original's word: the ten weapon codes, the Torch and Flaming masks tested with AND, `with`, `weapon_moveset`, `overlays`, `Display`), `ClipTable`, `MotionSet::assemble` (base chain from the root; the weapon moveset under each ancestor, children first, with its path below the ancestor; the overlays from the last hit up to its ancestor, then earlier hits), `table_clip`, `clip(base, config, action)`; the name-walk `clip` removed

## 2. Bodies

- [x] 2.1 `exa_anim`: `Animator::config` (base = the race's own moveset), `Motions::new`, `Motions::table` (assembled once per base and config, shared by clones, a missing base warned once) and `Motions::clip(anim, action)`; `progress` and the frame-clocked bodies through it; `plays_default_alone`; `DEFAULT_MOVESET`
- [x] 2.2 `exa_character::held`: `HeldMoveset::config`; `choose(hands, claws)` gives the configs (fists none, a main-hand light One Handed + Torch, a shield + Torch beside a light); `choose_held_movesets` writes the config and keeps the base
- [x] 2.3 Every lookup through the table: `limp` (clip tick, gait floor), `balance` (get-up choice, its idle), `trace`, `exa_combat` (swing progress, a struck fighter's idle), `player` (attack log); `TorchHand::config`

## 3. Tool

- [x] 3.1 `exa-tool clips <base> [config]` (hex or decimal) and numbered movesets with resolved parents in `exa-tool rsq`

## 4. Tests

- [x] 4.1 `rsq`: parents (#3 under #2, not #1; own name and case-mismatched parents are roots); the base chain from the root (A/B/C from Default/Mid/Leaf; base case ignored; a missing base none; an undefined action none); the weapon moveset under every ancestor (golem [1, 2, 3], human [1, 0, 0], polearm and 0x2 add nothing); children before grandchildren ([0, 3]; without the child [2, 0]); overlays from the weapon up (shield + Torch [1, 2, 3, 0]; One Handed + Torch and + Flaming [1, 0, 3, 0]; Torch alone none; the root's own Torch unseen); configs (0x10010001 → One Handed; both overlays for either mask; Display)
- [x] 4.2 Corpus (skipped without `EXANIMA_DIR`): the 32 parents of the installed tree; the 9 race bases of `races.rdb`; all 234 tables of 9 bases × 26 configs play every action; 162 differ from the old name chain, for humans only the 9 shield/flaming tables with the Torch overlay (4 actions each); the movesets used (golem sword [0, 1, 14, 15], golem shield [0, 2, 15, 16], golem polearm [0, 13, 17], ogre sword [0, 1, 27, 28], Herath/Gobbler sword [0, 1, 22, 23, 24], Harvester [0, 1, 22, 23, 24, 25]); the deepest base chain 4
- [x] 4.3 `exa_anim`: a body's clip follows its config, the table is built once and shared, a missing base plays nothing; `held`: the configs (0x1, 0x4, 0x30, 0x20, 0x10, 0x40, 0x100/0x200/0x300, 0x3, 0x10010001, 0x10010200), the race kept on a change of hands; corpus: every weapon item's config resolves its stance and attack clips (136/32/19/18/3); `torch`: the hands' configs and, corpus, a held torch's table equals its moveset's chain; `exa-tool`: configs in hex or decimal

## 5. Verification

- [x] 5.1 `exa-tool rsq` and `exa-tool clips` against the installation (Fael 0x1: Walk from Demon A01 #23, attacks from One Handed #24; Default 0x10010100: Walk, Cmbt - Idle, Attack Mid from Torch #7, the guard from 1H & Shield #2)
- [x] 5.2 In game, light runs through `gpu_run.sh --gpu 4000` (`--no-gi --no-supersample --size 960x540`): `--weapon ub05ssworda --still-opponent --at 0,0,1.7 play arenao1` shows the player in the One Handed guard and the duellist in the low One Handed combat idle, as before; with `--press LMB@60` the log reads "attack: Attack Up in Default with One Handed" and the swing plays; no panic or validation error. `play exanima01` (the torch) ran out of GPU memory at load (about 2.4 GB free on the card), so the held torch's clips rest on the corpus test that its table equals the "Torch" moveset's chain
- [x] 5.3 Remove probes and scratch files from the tracked tree (none were added)

## 6. Documentation

- [x] 6.1 `docs/formats/RSQ.md` (Motion set: the tree, the clip table, the configs, Greatsword under 2H Light), `docs/PLAYER.md` (Moveset from held items), `docs/PLAN.md` (Phase 4), `crates/exa_formats/README.md`, `crates/exa_anim/README.md`, `crates/exa_character/README.md`, `crates/exa_tool/README.md`
- [x] 6.2 `openspec validate --all --strict`
- [x] 6.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR` (503 passed, 3 ignored)

# Tasks

## 1. Readers

- [x] 1.1 Action sets, actions, default table, set table with layer and restore, focus stack.
- [x] 1.2 Menu frame: dead zone, analog bits and cursors every frame, repeats and fast quirk, keys, exit hook, priming.
- [x] 1.3 Directional pick at the GUI unit with the engine's normalisation and double-precision alignment.
- [x] 1.4 Gameplay frame: move and run flag, power-bar and dialogue steps, look at the orbit rate, aim, stepped zoom, power-tree hold, command word, key bits, sprint, rumble.

## 2. Wiring

- [x] 2.1 The plugin samples the first pad; controller mode follows the Controls page.
- [x] 2.2 The player reads the pad for steering keys, crouch, attack, stab, weapon swap, combat toggle, combat aim and the chase camera's look and zoom; keyboard and mouse unchanged.

## 3. Docs and check

- [x] 3.1 Update `docs/PLAYER.md`, `docs/UI.md`, `docs/ARCHITECTURE.md` and the `exa_game` and `exa_ui` READMEs. `docs/PLAN.md` is the integrator's.
- [x] 3.2 Corpus, skipped: no format field changes. Remote tests and clippy for `exa_game` and `exa_ui`, and `openspec validate --all --strict`, pass.

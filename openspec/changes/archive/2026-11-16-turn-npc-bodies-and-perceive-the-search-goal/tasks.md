## 1. Read the original

- [x] 1.1 The AI tick's heading block after the state and the look snap
- [x] 1.2 The combat tick's look writes, the charge, the sweep's head tracking and the stab start
- [x] 1.3 The chase's grip mode
- [x] 1.4 The search's pick and release, the goal entity's assessment, the target pass's goal branch, the integration and attention tests

## 2. Port and wire

- [x] 2.1 Body directions written into the locomotion request and taken back
- [x] 2.2 The heading block and the look snap; the duellist turns by its body
- [x] 2.3 The head tracking from the sweep; the chase's grip mode
- [x] 2.4 The goal entity in the brain, the target pass and the search

## 3. Tests and checks

- [x] 3.1 Live-system tests: a duellist turns by its heading on the AI chain; its body trails the heading under the movement decision; a stab tracks the head; the chase sets grip mode 1; the frozen tick; the search waits for its goal entity under the real AI tick
- [x] 3.2 fmt, clippy -D warnings, the workspace tests, openspec validate, the play build
- [x] 3.3 Worker GPU scenes (duel, search), the golden suite and determinism

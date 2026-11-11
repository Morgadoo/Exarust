## 1. Trace

- [x] 1.1 Read the striking part's hit-event handler and the swing routine's window writes on the strikers against the assembly
- [x] 1.2 Read the character's combat-target writers (player auto-target, AI combat entry) and readers (starters, parry check)

## 2. Port

- [x] 2.1 Fists through the hit event (classifier on the hand, struck fist as a clash)
- [x] 2.2 The hand's strike-chain bit from the window
- [x] 2.3 No sweep for fists on a body part
- [x] 2.4 CombatTarget written by the character tick, read by the starters' aim and the parry check
- [x] 2.5 Unit tests

## 3. Check

- [x] 3.1 fmt, clippy -D warnings and tests on the worker; play build
- [x] 3.2 GPU grid: exanima01, plain duel and unarmed duel, two runs each, against r162
- [x] 3.3 Update the exa_combat, exa_game and exa_ai READMEs

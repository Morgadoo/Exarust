## 1. Read the original

- [x] 1.1 The oldest body converter's count setter and seed draw; the operative converter's tables
- [x] 1.2 The object script run and the counted list
- [x] 1.3 The search's gathering mask and the container add's size lookup
- [x] 1.4 The loadout form's cost and the weapon points routine
- [x] 1.5 The grade routine's callers (the tournament prize at −1)

## 2. Wire

- [x] 2.1 `Store::parse_with_generator`, the count setter, the save load's twister
- [x] 2.2 `Operative::script` through `counted_list`
- [x] 2.3 `Loot::of_payload` through `Record::has_any`; the sizes through `flagged_section`
- [x] 2.4 The loadout cost through `weapon_points`

## 3. Tests and checks

- [x] 3.1 Live-system tests: an older save's world objects upgraded on the story load (0D08, 0D07, 0D06), a class change refreshing a saved instance's appearance, the level sizes and loot from the live database, a won prize graded pristine when stowed; the one-handed weapon's cost
- [x] 3.2 fmt, clippy -D warnings, the workspace tests, openspec validate, the play build
- [ ] 3.3 GPU grid runs (owed; the card is on hold)

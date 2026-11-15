## 1. Settling

- [x] 1.1 The scene's take-back settles a non-dormant character for 83 ticks
- [x] 1.2 The story respawn's placement settles the player for 16 ticks
- [x] 1.3 A pending-respawn load places the player at the portal and settles it for 16 ticks
- [x] 1.4 Spirit Form's new body is built settling for 16 ticks

## 2. Freeze and held items

- [x] 2.1 The original's freeze (countdown 2, particle sets copied) on the cursor's lock release
- [x] 2.2 The kinematic bit: set by the freeze, cleared by the thaw, out of the level detection
- [x] 2.3 A held item takes its holder's park mark before the solver

## 3. Barricade

- [x] 3.1 The dismissal pick tests the wall's box by separating axes

## 4. Checks

- [x] 4.1 Tests on the live systems (presence, load, body build, the physics plugin's step)
- [x] 4.2 fmt, clippy, the workspace tests, openspec validate, the play build

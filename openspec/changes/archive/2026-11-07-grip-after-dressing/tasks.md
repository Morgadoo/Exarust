## 1. Find

- [x] 1.1 Bisect round 85 (7f1cd24ab good, cebb3cb29 bad)
- [x] 1.2 Trace the held item and hand per tick; read the ground placement's logs

## 2. Fix

- [x] 2.1 The pending grip resolves before the builder; the builder waits for a known holder
- [x] 2.2 A held item's body waits for its holder's body; the torch too
- [x] 2.3 Bodies are built from the skeleton's nodes only
- [x] 2.4 The ground placement leaves out the placed body's own held items
- [x] 2.5 Tests

## 3. Check

- [x] 3.1 GPU grid: the player at idle and swinging, the opponent, the exanima01 torch, a hub fighter
- [x] 3.2 Two runs each of the arena duel and exanima01 agree for every body

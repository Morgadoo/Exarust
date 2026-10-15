## 1. Flood

- [x] 1.1 Wall openings per side from planes A and C (stubs, corner flags, doors, special tiles)
- [x] 1.2 Depth-first angular flood with the stack limit, reach and facing tests
- [x] 1.3 Cell light lists: union on re-entry, twelve lights per cell

## 2. Links

- [x] 2.1 Node test (cell, spill neighbours, sphere pass) before the link weight
- [x] 2.2 Lights flooded in creation order when added or moved; terrain exempt

## 3. Checks

- [x] 3.1 Unit tests on synthetic rooms: walls stop light, doors pass a beam, twelve per cell
- [x] 3.2 exanima01 log room lit as the original (frame mean 49.1 against 49.1), about twice the frame rate under the same load

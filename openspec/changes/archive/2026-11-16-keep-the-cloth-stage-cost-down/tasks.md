## 1. Measure

- [x] 1.1 The stage's split by piece on the worker (`ClothDebug::last_split`)
- [x] 1.2 The base and the lane interleaved on the worker, the pool's dispatch alone

## 2. Change

- [x] 2.1 Keep the links of a body whose parts stay in their cells
- [x] 2.2 The benches on the solver's lists
- [x] 2.3 The steps sent to the pool as they are planned; the staged list without a map
- [x] 2.4 The hook's lists kept across ticks, each cell resolved once, one pre-test for the particle mode
- [x] 2.5 The end of tick over the staged list

## 3. Check

- [x] 3.1 The hair checksum and every cloth and physics test unchanged
- [x] 3.2 A changed body that did not leave its cells keeps its links (unit test)

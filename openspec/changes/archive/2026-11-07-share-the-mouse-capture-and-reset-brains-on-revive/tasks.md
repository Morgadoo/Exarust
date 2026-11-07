## 1. Find
- [x] 1.1 REA: the capture routine, its aim point, snapshot and pointer pin
- [x] 1.2 REA: the behaviour's cursor gate and the cast hook asks
- [x] 1.3 REA: the space query's AI caller (method slot 26, the revive pass)

## 2. Port
- [x] 2.1 One capture for the orbit and Cursor Look, the per-frame centring, the aim-point warp
- [x] 2.2 Cursor Look turns the cursor follower's target; the camera's align flag and the pointer pin
- [x] 2.3 The revive pass's brain reset with the space query
- [x] 2.4 Tests

## 3. Check
- [x] 3.1 GPU: exanima01 and the arena duel lockstep traces identical to main; Cursor Look and orbit holds scripted
- [x] 3.2 fmt, clippy -D warnings, tests, play build, openspec --strict

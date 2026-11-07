## 1. Find
- [x] 1.1 REA: the blast's contact slots, the mesh walk, the push-out and limb callbacks, the part slot's knock
- [x] 1.2 REA: the sim-object walk in the assembly (precision, no life test), the cloth's wake and rustle envelope
- [x] 1.3 REA: the ground and nearest-mesh picks' callers among the casts and their accept callbacks

## 2. Port
- [x] 2.1 The per-part contact against the node mesh, the sphere stand-in for a part without one
- [x] 2.2 The cloth stir with the contact scale
- [x] 2.3 The cast picks: throw, long door, summoning, barricade placement and dismissal
- [x] 2.4 Tests

## 3. Check
- [x] 3.1 GPU grid: the AI's blast striking the player part by part, the throw from the ground pick, the summon on the picked floor, the barricade dismissed under the cursor (new against main)
- [x] 3.2 Lockstep: exanima01 and the arena duel identical to main; the blast and throw scenes repeat tick for tick
- [x] 3.3 fmt, clippy -D warnings, tests, openspec --strict

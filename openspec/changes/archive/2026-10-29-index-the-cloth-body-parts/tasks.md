## 1. Bench

- [x] 1.1 Add a CPU bench of fifteen hairs among 4 800 sleeping bodies of ten parts printing the parts split and a particle checksum; record the baseline (parts 3.2 ms a tick on the build worker)

## 2. Index

- [x] 2.1 Add `cloth_parts::PartCells`: re-bucket changed or new bodies, record walk ranks, sweep gone bodies, part-to-body lookup, sorted cell reads
- [x] 2.2 Run the plan before the gathering, gather only the cells around the cloths, find the hair's owner through the index, clear it while no cloth exists
- [x] 2.3 Log the bodies re-bucketed and parts gathered with the `parts` split
- [x] 2.4 Test the index against a fresh full re-bucket; check the benches' checksums are unchanged (parts 0.08 ms a tick)

## 3. Hair on by default

- [x] 3.1 `hair_cloth_on` defaults on (`EXA_HAIR_CLOTH=0` turns it off)
- [x] 3.2 Update the crate READMEs and docs/DEBUGGING.md with the GPU check

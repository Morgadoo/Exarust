## 1. Format

- [x] 1.1 Parse the bone body chunk's links, foot contacts and thorax pins
- [x] 1.2 Document the layout and the simulation rules in RSQ.md

## 2. Simulation

- [x] 2.1 Tetrahedron parts with edge constraints, built from part mesh boxes
- [x] 2.2 Verlet integration at 83 Hz with speed-banded velocity retention
- [x] 2.3 Hard-radius links in the parts' rest frames, split by inverse mass
- [x] 2.4 Sphere contacts against static geometry with the slip-limit friction
- [x] 2.5 Unit tests: parts come to rest on the ground; links hold parts together

## 3. Game

- [x] 3.1 `GoLimp` turns a posed body into a ragdoll and releases its controller
- [x] 3.2 Placed corpses go limp; knocked-down fighters go limp
- [x] 3.3 Viewer `--limp` and `--at`; verified by screenshot

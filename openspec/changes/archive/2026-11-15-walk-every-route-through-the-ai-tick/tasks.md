## 1. The walk

- [x] 1.1 Port the follower, the walk, the route stop and the soft stop from the instructions
- [x] 1.2 Keep one route per character and run the walk after every state of the brains' tick
- [x] 1.3 Move the roam, the flight, the chase and the follow onto the route; remove the per-state walkers
- [x] 1.4 Stop the route of a downed body and on the knocks

## 2. Proof

- [x] 2.1 Unit tests against the instructions' numbers (carrot, stops, slide, gate, head look, blocked line)
- [x] 2.2 A test with the real AI plugin's brains' tick walking a roam leg to its end
- [x] 2.3 Specs and READMEs

## 3. Checks

- [x] 3.1 fmt, clippy -D warnings, the workspace tests, `openspec validate --all --strict`, the play build

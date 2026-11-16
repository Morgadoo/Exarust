## Why

The `clothbody` lane moved the level's cloths into the bodies' tick at the
original's points (the cloth's own step before the solver, its body
contacts in the solver's object stage, its after-collision pass and level
contact after it). The CPU bench `cloth_cost` got slower than the round 233
base (15 awake hairs: 0.157 → 0.247 ms a tick; among 4 800 sleepers, the
parts split 0.089 → 0.190 ms and the stage 0.224 → 0.464 ms), and the
owner's rule is that a change must not lower FPS. No behaviour changes:
the hair checksum (5a45a5c364b1c3b3) and every cloth and physics test stay
identical, and the step order stays.

## Evidence

No new reading of the original: the order is the clothbody lane's (the
grid sector step's object step after the bounds pass, the object–body
contacts after body–body detection, the after-collision and level passes
after them). Measured on the build worker (release, interleaved runs of
the base and the lane, medians), split by piece with the new
`ClothDebug::last_split`:

- The awake bodies were re-bucketed twice a tick (the stage's refresh and
  the hook's re-bucket after the solver), each time unlinking every part
  with a linear search of its cell's list.
- The bench's no-solver stand-in walked every body into two lists, a
  place map and a change mark; the game's hook gets the solver's lists.
- The no-solver stand-in counted the contacts twice in the stage's time.
- A particle-mode cloth made the pre-test twice (the hook's filter and its
  gather's own).
- The hook looked each linked part's body up in a hash map once per cloth
  and allocated its maps every tick.
- Each of the three pieces meets the compute pool (about 20 to 35 µs a
  meeting on the worker, mostly waiting for its threads to wake); the base
  met it once.

## What changes

- `exa_world::cloth_parts::PartCells`: a body whose parts all stay in their
  cells keeps its links (no unlink and relink).
- `exa_world::cloth_stage`: each awake cloth's step goes to the pool as soon
  as it is planned (inside one scope), so the threads wake while the plan
  goes on; the staged cloths are built in the plan, without a map or a
  lock; the hook keeps its slot map, cell lists and sleeper map from tick
  to tick and resolves each cell's parts to the solver's slots once; a
  particle-mode cloth's gather makes the only pre-test (its debug tags
  number the offered parts again, as the base's did); the end of tick runs
  over the staged list and puts the cloths back in the list's order.
- `ClothDebug::last_split`: the stage's six pieces in the last tick (index,
  plan, steps, the hook's bucketing, the contacts, the end), and the parts
  split is the bucketing only (the contacts count with the cloths, as the
  base counted them).
- The benches run the game's path: a stand-in for the solver's lists
  (split and ordered as `step_bodies` hands them, the hook called with
  them, no physics) between the stage's two halves, and print the split.

## Result

Worker CPU, release, ten interleaved rounds, medians (ms):

| bench | base f94d14cd1 | base, bodies changed | lane after |
|---|---|---|---|
| 15 awake hairs, tick | 0.117 | 0.145 | 0.144 |
| among 4 800 sleepers, parts | 0.079 | 0.080 | 0.037 |
| among 4 800 sleepers, stage | 0.182 | 0.180 | 0.175 |

The base's first bench leaves the characters' bodies unchanged; the game's
body stage marks every awake body changed each tick (the level bench and
the lane's solver stand-in both do), and with that the base costs what
the lane does. The rest of the gap to the base's first bench is the two
pool meetings the order adds (the body contacts inside the solver and the
end after it), which no change that keeps the order removes. The lane
before this change measured 0.228, 0.175 and 0.438 ms on the same worker.

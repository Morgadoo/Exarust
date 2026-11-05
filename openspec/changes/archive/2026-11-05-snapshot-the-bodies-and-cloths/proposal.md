## Why

Stage 4 of the simulation thread (`make-the-bridge-the-default`): the frame's writers of simulation data become work of the simulation, and the copies out become snapshots, group by group, to cut the copy cost and prepare the thread's release from lockstep (stage 6). Lanes adding fixed-step types also need an unmissable signal when they forget to register one.

## What Changes

- An unregistered type a moved system touches stops debug and test builds at the first step; play builds warn and skip the system.
- `SimFrameStart`: the fixed step's once-a-frame work. In one world it runs before the frame's fixed loop; behind the bridge it moves with the fixed step and runs at the first step after a frame began. The bodies' frame start (blows queued, tick counts reset) moves there: the frame no longer writes every awake body each frame.
- Cloths: after each step the stepped cloths' mesh positions go to the frame as `ClothMeshUpdate` messages; the frame's write-back applies the newest per mesh. `ClothInstance` crosses only when the frame builds one (a new copy-in kind).

Phase 3 (bodies: skeleton, physics, control) in docs/PLAN.md: the fixed tick's cost.

Evidence: no engine rule is added. Parity with `--sim-digest` (one world against the bridge, step by step), the bridged body test, duel joint traces.

## Out of scope

- The other frame writers (the hand, GUI drops, hotkeys, limp, rise, hits and bites, the viewer's debug), the transforms' and the hierarchy's copies, freeing the thread (stage 6).

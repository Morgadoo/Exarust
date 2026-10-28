## Why

Round 24 made the level's cloths gather body and level contacts every tick. On exanima01 that stalled the game: loading took minutes with 10-second frames, and the level then ran at 4 FPS. Two causes were measured on the RTX 3090:

- A units error: a cloth's bound radius was its centimetre half-extent multiplied by 100 again, so every body part in the level passed the cloth's pre-test and became a swept contact.
- With that fixed, the 14 cloths of exanima01 still never slept (the walls they hang against kept pushing them) and their sweeps cost about 55 ms a tick (2249 swept spheres, about 15 600 level triangles).

## What Changes

- The cloth's bound radius is its half-extent in centimetres (scaled by the node), not a hundred times that.
- The body-part list for the object stages is built once a tick for every cloth.
- A sleeping cloth skips the gathering (it does not step, so its contacts were never read).
- The live contacts are opt-in (`EXA_CLOTH_CONTACTS=1`, the `ClothContacts` resource) until their cost and the cloth's sleep under contact are matched to the original. The sector's object stages stay tested.

Phase 3 (bodies) in docs/PLAN.md.

## Evidence

GPU runs of exanima01 (RTX 3090, `--no-gi`): before the fix, 10.4-second frames in the objects stage and 4 FPS after the load; with the radius fixed, 55 s to load and 4 FPS; with contacts opt-in, 18.1 s to load and 60 FPS (round 22: 17 s, 60 FPS).

## Out of scope

The original's cloth sleep under contact and the cost of its contact sweeps.

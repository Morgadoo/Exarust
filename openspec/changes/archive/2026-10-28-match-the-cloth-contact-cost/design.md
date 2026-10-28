## Context

Each cloth contact rule here follows the original's instructions: the float order, the unordered branches and the constants. Exarust keeps everything in the cloth's own frame and in centimetres. The original tests in each mesh's frame and, for bodies, in the world. For rigid unit-scale frames the two agree.

## Decisions

- **One piece per static record.** The original's stage visits one static mesh at a time. Each mesh is tested against the particle's sphere (or sweep) with its node box, then against every triangle. Exarust's piece box is the box around the triangles handed over. That is a tighter but exact cull: a particle that misses it reaches none of them.
- **Closest point.** parry's triangle projection stands in for the mesh's own edge walk. This is a representation difference; the point is the same for finite input.
- **The sleeping cloth's skip.** In the original the static stage runs for a sleeping cloth too. There its particles sit where the last tick left them, and the slide returns a pushed particle to that point (or to the same partial point every tick). So a sleeping cloth over still geometry is a fixed point after one tick. Exarust skips the stage while the cloth's frame is unchanged and no awake body reaches it.
- **The body cull box.** The original tests against the cloth node's world box. Exarust tests against the box around the particles in the cloth's frame. A sphere outside that box can neither push a particle nor cross a cloth triangle.
- **Default on.** The cost now has the original's bounds. A resting cloth sleeps within its 32-tick countdown. A sleeping cloth costs one bound test per awake body part. An awake cloth costs one plane distance per particle and triangle of each mesh piece whose box the particle reaches, with the triangles limited to the cloth's bound. The original pays for every triangle of the mesh. The integrator checks this on the GPU.

## Verification

The tests cover each rule with concrete numbers. Check the frame time on the GPU:

- `play exanima01 --perf-log 5` (contacts on by default) must hold 60 FPS and load in about 18 s, as with `EXA_CLOTH_CONTACTS=0`.
- `play arenahub`: same frame rate with and without the switch.
- A banner should settle and stop within a second of the load.

If the frame time is not flat, `EXA_CLOTH_CONTACTS=0` restores round 25's behaviour.

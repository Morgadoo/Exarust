# Proposal

## Why

The third DeepSeek pass on the content stream and texture units (lane
`streaming3`, docs/PLAN.md Phase 2, Static world: texture loading and the
content stream) added cube-map uploads, the level-transition marker as it
is stored, the placed objects' flag words, the streamed character's stage
node, the bias rewrite on a quality change, and rewrote the texture
loader's pass. Its review checked each rule against the original's
instructions and found two wrong: the loader does wait behind queued
uploads (it suspends its own thread until the upload pass wakes it; the
lane had it waking the main thread instead and never waiting), and a
placed object's static bit marks only its root node (the lane made the
whole object static, which took arenao1's tableware, pegs and bundles out
of the solver).

## What Changes

- content-streaming: Texture loader queue, passes, marks and upload pass
  say which record the priority path finds and when the loader waits;
  Player focus compares a marker's entrance and previous level with the
  session's two variables; new Transition markers, Placed object flags and
  Streamed character handover.
- texture-decoding: new Cube uploads; Texture LOD bias also covers a
  settings change rewriting the loaded materials' bias.
- Code: the loader's wait restored with the held record
  (`TextureStream::wants_to_yield`), the invented main-thread wake
  removed, the placed objects' bodies registered again.

## Evidence

The loader loop, the priority load and promotion, the upload pass, the
thread's self-suspend and wake helpers, the upload routine's target
switch, the marker check and the placed-object reader in the streaming
decompilation pack and the full disassembly; the 54 shipped transition
markers read from the levels' own item databases.

## Out of scope

A Bevy driver for the content stream, the reader registry and provider
create variants, the terrain request path, block-compressed and 1D/3D
cube uploads, what a placed object's root flag does to its bodies, and a
live level change through the markers.

# Proposal

## Why

docs/PLAN.md, Phase 3 (bodies: skeleton, physics, control), the player's
controls: the playerctl10 lane (SG_PlayerControls unit routines) made the
lift refuse a placement marker the item would overlap, modelled the use
cursor's begin and end, and gave the deferred drop its state-word gates
and frame-copy gate. Its review checked each rule against the
executable's instructions, kept all three, made the marker overlap's
sphere gathering lazy and corrected the use end's action word.

## What Changes

- Marker overlap: a placement marker named for the held item is refused
  when the item, moved onto the marker's frame, would overlap another
  body (the item's collision spheres against the other bodies' part
  spheres, the player's own body excluded); the search goes on with the
  next marker. Wired in the live lift; the spheres are gathered only once
  a matching marker is met.
- Use cursor begin: an operative section of class 0, kind 0x11 opens the
  GUI item its word names; any other (or none) lifts the cursor only while
  play is on, with state word 0x100 and shared cursor mode 0x40.
- Use cursor end: only event 0x80 ends it, answering 0x81 (others answer
  1); the use runs only over the world; the player's action word becomes
  4, the use state clears and the cursor mode returns to 4.
- Deferred drop: no armed bit (0x1000) frees the node; a node without
  physics is the content error and resets the word to 0x1000; a placed
  node takes the condition word and the −100000 cm limit, copies the
  body's frame only when the word lacked 0x2000, and ends at 0x2000.

## Evidence

The placement routine's marker loop (marker matrix copied into the body,
the dynamics overlap query, the matrix restored on overlap and the loop
continued), the dynamics overlap query itself (bodies in the nine
neighbouring 600 cm cells, same-owner and collision-mask filters, a
disabled flag, the pair narrow phase, then the children recursively), the
use cursor's begin and end callbacks and the deferred drop task, read from
the disassembly.

## Out of scope

The overlap query's narrow phase against triangle meshes and its mask and
owner filters (sphere pairs against Exarust's other bodies stand in), the
use begin's and end's wiring into the right press (the models are tested
functions), the deferred drop's wiring, the placement sweep's per-sphere
support-plane pull-back and the item GUI windows.

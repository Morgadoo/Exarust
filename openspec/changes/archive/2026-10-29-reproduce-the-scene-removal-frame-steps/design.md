# Design

Evidence: the REA (Ghidra) listing of the scene frame, the removal queue
writer and the scene-bounds pass; no format page changes (the scene graph
has no file format here).

Open questions:

- What the node destructor's deferred mode does beyond setting the deleting
  bit: it belongs to the node unit; the model sets the bit and lists the
  node on the free list (labelled as a guess in the code).
- What the frame does with the `_World` sector after draining the ring (one
  call on a scene field): unknown, not modelled.

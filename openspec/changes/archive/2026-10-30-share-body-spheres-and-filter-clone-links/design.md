# Design

- The sphere list is an `Arc<Vec<(Vec3, f32)>>`. Nothing writes to it after a body is built, so the original's copy-before-write is never forced; the pair stage's temporary take-out swaps the `Arc` rather than the list.
- `Ragdoll::copy(drop_runtime_links)` builds both copies. It counts the links kept before each index and maps every part's range and leg-limit index through that count, so the plain copy is the identity and the clone slot's ranges hold exactly the surviving links.
- The clone slot copies the whole state rather than the original's subset (frame, flags, links, world links, spheres): the original's copy re-runs its setup, so the extra fields are overwritten there; no Exarust path re-runs a setup on a clone, and none calls the slot yet.

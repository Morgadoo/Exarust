# Design

Formats page: none (no file format changes); behaviour is in
`crates/exa_assets/README.md` and `crates/exa_world/README.md`.

## Decisions

- The terrain tile spawns only after its textures arrived instead of
  binding the placeholder: its splat bind group and composite bake read
  the images once, and waiting costs nothing on the main thread because
  the loads start as soon as the tile's materials are known.
- A caller that loads the level in one call has no frames for the loader,
  so it finishes a waiting record itself, as the original's priority wait
  does for a texture needed at once.
- Exarust queues terrain loads only while its texture loader runs (the
  manager's streaming switch); tools and tests without it decode
  synchronously as before.
- The stream is held behind a mutex only so `LevelLoad` can stay in a
  resource; it is reached with `get_mut`, never locked.

## Open questions

- Whether a placed object of another slot counts as built or missing: the
  builder is not seen to filter by slot, so it counts as built (guess,
  labelled in code).
- The flags at the room-mode word beyond its two low bits are not traced.

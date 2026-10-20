# Proposal

## Why

The frame profiler showed play mode's render thread spending 29 ms a frame
on Bevy's own point-light shadow maps: every light with a character linked
re-rendered all six faces every frame, about 111 extra views, 113 runs of the
camera schedule and 743 command-buffer submissions a frame. The original has
no second shadow system: characters' body parts are casters like any node,
and a moving caster dirties only the faces it was and is in.

## What Changes

- Characters' skinned meshes cast into the cached cube maps, posed by their
  joints in the caster shader (joint world transform × inverse bind pose,
  computed once a frame per character drawn).
- Each light keeps its casters with the cube faces each one's bounding
  sphere touches (the original's link face mask). A face draws only the
  casters that touch it.
- A caster linked or unlinked dirties the faces it touches; a caster that
  moves dirties the faces it touched and touches now. A moving static mesh
  no longer dirties all six faces.
- Bevy's point-light shadow maps stay off.

## Capabilities

### Modified Capabilities

- `rendering`: cached cube shadows for every caster.

## Impact

- `exa_render::cube_shadows` (`CubeCasters` with face masks,
  `sphere_faces`, skinned draws and joint buffer), `cube_shadow.wesl`.
- `exa_world::shadow_links` (caster lists with masks, `dirty_moved_casters`,
  no Bevy maps).
- `exarust::column` (the renamed caster list).
- `docs/RENDERING.md` (Shadow links, Cached cube shadows), `docs/PERFORMANCE.md`.
- Evidence: `context/re/optimizations.md` §4.4 (needed and dirty faces, a
  casting node moving ORs its old and new masks), `context/re/shadows.md`
  §3.4 (casters per face by the link's face mask).
- Out of scope: the original's needed rule by visible linked receivers
  (Exarust keeps "the face's pyramid meets the camera's view").

## Phase

Phase 8, performance.

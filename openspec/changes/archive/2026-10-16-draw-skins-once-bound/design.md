## Context

Bevy picks a mesh's pipeline layout from its vertex attributes: a mesh with
joint indices and weights gets the skinned layout. It picks the bind group
from the entity: the skinned one only if the entity has a `SkinnedMesh` whose
joints were extracted. Exarust binds skins in `exa_anim::bind_skins`, after the
character's body parts are tagged. That happens at least one command flush
after the meshes are spawned, and later when the skeleton or apparel arrives
in a later frame.

## Decisions

- **No `Mesh3d` until bound.** `SkinnedPart` holds the mesh handle, and
  `bind_skins` inserts `Mesh3d` and `SkinnedMesh` in one command, so there is
  no frame with one and not the other.
  - Rejected: hiding unbound meshes through `Visibility`. That conflicts with
    the hiding of body parts under apparel, which also writes `Visibility`.
  - Rejected: clearing `ViewVisibility` after Bevy's visibility checks. Bevy
    0.20 tracks the previous frame in that component, and the per-view
    visible lists are built from it within the same system set.
- Readers that need the mesh before binding take the handle from
  `SkinnedPart`. These are the limp body's part boxes and the
  covered-part query, which matches `Mesh3d` or `SkinnedPart`.

## Risks / Trade-offs

- A skinned mesh whose character has no skeleton is never drawn. No shipped
  model does this, and drawing such a mesh failed validation anyway.
- Shadow links and cube shadows see a skinned mesh only from binding on.
  Their `Added<Mesh3d>` filters pick it up then.

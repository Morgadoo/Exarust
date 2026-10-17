# `exa_assets`

The bridge between `exa_rpk`/`exa_formats` and Bevy: the `Game` resource
holding `GameData`, conversion of parsed data into Bevy assets, and
installation discovery (`--game`, `EXANIMA_DIR`, `.env`).

**Phase:** 0 (started), 2 (asset source `exanima://`, loaders, direct BC upload, streaming).

Model nodes carry their rigid body with the physical divisor of their first
mesh part's material (`ModelNode::rigid`, `NodeRigidBody::mobility`), what
the solver registers a prop with.

Materials that show wound blood (`docs/RENDERING.md`, Blood): the human
skin, painted and run-time apparel and factory items, any record selecting
dirt mode 4 with a diffuse map, and a bleeding race's body-model materials
whose names start with `s`; each binds the speckle texture `spatter01`
(`ModelLoader::spatter`).

Physique (`morph`): `ModelLoader::morphed_model` / `morphed_job` build a
character's variant of a race body or garment at its `BodyMorph` (race
stem and record physique): each node's share of the race's morph set is
transferred once per race and model (a garment piece from the frame of the
base body part its node names), then the meshes are morphed (normals and
boxes refitted) and the body parts' translations moved with their joints
before skinning; the body's parts are skinned on the morphed shape, a
garment from the unmorphed body. Weights that change nothing, and races
without a morph body, return the shared model. See
[`docs/formats/RFC.md`](../../docs/formats/RFC.md) (Morph sets).

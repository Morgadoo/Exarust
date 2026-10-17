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

# Design

The model keeps a light's alias copies as nodes (`SceneNode::aliases`) and
the sectors' `LightAlias` entries; the review adds `LightAlias::owner` so
the membership rebuild can test ownership as the original does, and a light
leaving the scene releases both representations.

The membership rebuild is written as the original's three stages with its
signed index map, including the quirk that a removed first slot with no
replacement maps to 0 and is read as kept by the bucket remap. Chain
indices past the map are read as kept in place (the original would read
past its map; the model's chains never reach there).

The state word `SceneNode::light_state` is set to 1 by `Scene::refresh`
like the original's transform update; the node passes take either it or
the older `moved` marker as "just moved" and clear both.

The shadow map's needed faces (`shadow_faces`) and linked faces
(`shadow_links`) are separate words; the alias test ORs the new and old
faces into the needed faces for a node with the split-bounds bit, as the
alias test does; the test's own answer stays with the caller (labelled a
guess).

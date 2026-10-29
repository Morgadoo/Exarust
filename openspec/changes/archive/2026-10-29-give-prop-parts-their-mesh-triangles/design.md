# Design

## Which mesh

The original's contact generation reads the touched body's scene node and
calls that node's sphere and swept-sphere queries, which walk its mesh's
kd-tree. Exarust copies the node mesh's positions through its index list
once per loaded model, behind an `Arc`, only for a node whose body
registers (a level's static geometry, drawn-only nodes and weapons keep
none, so load time and memory do not grow with the level).

## Order and cost

The triangles are visited in file order. The kd-tree walk would change the
order of hits (and so which later triangles see the push), not the set of
triangles that can touch a static sphere. The pre-tests bound the work to
spheres that reach the node box; a per-part kd-tree walk (the
`kd_tree` module has the swept walk but not the static sphere walk) is the
cost cut to make before character meshes are handed over.

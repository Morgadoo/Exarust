# Design

The pick stays a decision over plain inputs (`hover_pick` takes the body's
hit, the nodes' distances, the hand flags and the worn list). The live side,
`exa_game::figure_pick`, measures those inputs on the figure's Bevy meshes.

- **Nearest hit.** The original passes one running distance through every
  ray callback. A mesh answers only a face nearer than that distance, and on
  a hit it writes the face's distance back. The body starts from the largest
  float. The nodes start from the body's hit, or from the ray's own length
  when the body was missed. So a node hit is "nearer than the best so far".
  A node's own nearest face, compared with the running best, gives the same
  answer, and it is what `PickNode::distance` carries.
- **The face test.** This is the meshes' default ray callback, an
  unnormalised barycentric test. The determinant must reach the float word
  `0x33ffffd7`, which makes the test one-sided. The barycentric weights are
  bounded by the determinant, and `0 < t < best`. Exarust's meshes are drawn
  with the front faces counter-clockwise. The test meets the faces that turn
  toward the camera, as the body's hit at about 3.9 m (the camera is 4.1 m
  from the figure's point) confirms.
- **The per-face table.**
  - The two appends write their tag into every face they keep (the cull
    append writes it only into the surviving faces).
  - The renumber pass permutes the table with the faces.
  - The dressed-mesh builder tags each garment's slot parts with the worn
    entry's item id, and the body's parts with 0.
  - Exarust's outfit carries models, so the tag is the garment's number.
    The pick maps the garment's model to the first worn entry worn as that
    model.
- **Skinned positions.** The original skins the merged mesh's positions on
  the CPU just before the test. Exarust reads the run's current skinned
  positions through the joints (`blood::skinned_positions`).
- **Slots.** The original reads the node's item and looks up the first
  settled node up its chain among the doll's attached items. Exarust finds
  the slot from the mesh's first ancestor that is a striker (`Blade` slot
  First → 1, Second → 2), the figure's shield root, or a `FigureHeld` root.
  The item is that hand's record entry.
- **The release.** Over a slot, the scheduled take-out (mode 7, record
  cleared, the off hand's weapon or light promoted) runs, and the player's
  pack drops its held entry when the item went elsewhere. The player's pack
  is linked to the view: its drop showed the held entry again under the
  pointer, so the listed copy loses its held mark and moves there. Exarust
  matches the pack's held entry by item, because its icons are per entry.
  A garment comes off the worn list, the underwear rule runs, the outfit
  loses its garment, and the player's pack is charged the item's bulk (the
  linked drop added none).

Open questions:
- A drop back on the view itself is taken as a cancel; the drop routine's
  answer for its own payload was not traced.
- The view's control byte that keeps the doll's hands is not named.
  Neither has a running caller.

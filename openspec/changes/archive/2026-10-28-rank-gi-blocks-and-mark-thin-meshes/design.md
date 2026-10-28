# Design

- The residency is a plain resource over layout indices, so its rules are
  tested without a GPU. Its distance follows the assembly: subtract,
  clear the sign bit, subtract 2400, replace by 0 only on an ordered
  "below zero", square both axes and add the X square to the Z square.
  The sort swaps on an ordered "greater", so NaN never moves.
- Unbuilt blocks keep their old distance (0 at first), so they sort ahead
  and the original builds them in list order, not nearest first. Exarust
  keeps building the camera's block first (the renderer holds one block
  and a load must be lit where the player stands) and follows the
  residency's order for the background builds.
- The texture handoff is computed every frame but only matters to a
  renderer holding several light textures; Exarust draws the camera's
  block alone.
- A background build gives way to a camera block that needs building:
  dropping the task cancels it and the block stays unbuilt, so the
  residency names it again.
- Thin meshes: the mesh class's set-up routine walks every surface and
  sets its thin byte from the material (`class & 0x4000`, or the opacity
  slot with opacity > 0.75, ordered); the splat reads surface 0. The
  model reader's first material group stands in for surface 0.
- The top-slice flag rides in `sun_flags.z`, which was always 0, so the
  uniform keeps its size and the naga layout test needs no change.

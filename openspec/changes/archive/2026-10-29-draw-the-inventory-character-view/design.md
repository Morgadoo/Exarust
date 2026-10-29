## Context

The GUI is a CPU canvas shown by one UI image node; the 3D world is drawn by the scene camera. Reading the figure back into the canvas every frame would cost a readback and a full re-rasterisation.

## Decisions

- The figure renders on the GPU into an image twice the view's size, shown by its own UI node just under the GUI canvas, so dragged icons and cards still draw over it. The painter clears the canvas over the view and the camera clears to the colour the canvas had there, so the figure stands on the window's own client.
- The figure is a copy of the player's body far below the level on render layer 4096, its meshes marked out of the shadow links before they run, with its own directional light; everything is despawned when the inventory closes.
- The drop runs as a message from the panel input to the view's system, which ends the source's drag (the pack's linked panel is the view).

## Risks

- The background is a guess (the original draws its kept still there).
- The figure copies the player's outfit and weapon, not its torch.

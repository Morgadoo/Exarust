# Design

- **One knock helper.** The blast's and torrent's character hits differ
  only in the mass term and energy loss; `knock_and_pins` holds the shared
  knock (length summed `z² + (y² + x²)`, capped at 30, doubled) and the
  pin gain `(1 − k)·0.08` (double), 0 below zero, 0.008 above 0.08
  compared in double, a NaN kept.
- **Contact walks by body kind.** The original's character hit always
  takes the face walk with the limb callback; the sampled walk is chosen
  by the piercing flag only in the static dispatch. Exarust keeps one flag
  (`BlastSetup::piercing`) and applies the sampled rule in
  `strike_level` only.
- **Level sweep.** A swept ball from the chunk's or point's previous place
  along its move stands in for the engine's closest-face query; the
  contact is the ball's centre at the hit. The cost is one shape cast per
  live chunk or point a tick (at most 16 a blast, 384 a torrent).
- **Ribbon in world space.** The strips are built in the camera's space
  with the original's arithmetic and mapped back to world space so the
  scene renderer draws them; the torrent uses the world eye.

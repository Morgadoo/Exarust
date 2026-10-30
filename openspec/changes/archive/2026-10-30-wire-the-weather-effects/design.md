# Design

The weather stays a pure model (`exa_world::scene_env`) ticked once a frame
in `PostUpdate`; everything the tick asks for is done after it, in
`exa_game::weather`, which can see both the scene environment and the audio
crate. The model's one-frame loop is kept: the tick reads the rain
effect's volume and pitch, the effect's update (run with its draw, after
the tick) writes them, so the tick reads the previous frame's figures, as
the original's tick reads what the last drawn frame left.

**Rain draw.** The original's fragment program samples a copy of the frame
(`FrmBuf`). Bevy offers that copy in its transmissive pass
(`ScreenSpaceTransmission`, `Material::reads_view_transmission_texture`).
The scene cameras get the component while the effect exists, so the copy
and the pipeline variants only exist in rainy scenes, and the variant
change happens behind the loading screen (the effect is created with the
level's preset). The drops are a line list rebuilt each frame (at most 8192
vertices); the original's unnormalised byte colours are passed as floats.
Frame cost: one copy of the frame and one line draw. The original draws a
one-pixel line per supersample corner at window resolution; on Exarust's
2× target a one-pixel line would be half as wide after the resolve, so each
line is drawn twice, one target pixel apart.

**The world camera.** The rain and the environment's view take the scene
camera that is active and draws the default layer; the first scene camera
in the query could be the inventory figure's camera, which put the box
beside the figure (seen on the GPU: no streaks until this was fixed).

**Debug aids.** `EXA_RAIN_DRAW=0` hides the streaks; `EXA_RAIN_DEBUG=1`
draws them magenta and logs the box; `=2` shows the frame copy.

**Rain loop.** The voice is driven exactly as the original's tick does it
against the voice's own flag word; the model's start bit is synced from the
voice (active or restart pending) after each tick so a stopped loop can
start again.

**Thunder.** Played through the bank's set player, listener-relative at the
strike's position in metres. The original's position holds the view's
world position; reproduced as found (most claps fall beyond 30 m and are
silent).

**Arena ambience.** The arena index is the level's place in
`arena_match::ARENA_MAPS`, whose steady-bed flag is the setup's calm table
(read from the executable: arenao1, arenao3, arenao5). The setup reads the
rain *target*.

Open questions and how they were answered:
- Which figures feed the loop? The effect's update stores the fall and the
  drift at the two fields the tick copies into the voice's pitch and gain
  (REA, the update and the tick).
- Is the colour scale switch a setting? It tests the global pause counter,
  the same one the weather tick tests (REA).
- The box's turn sense: the stored matrix read as acting on column vectors;
  unverified (2° is not visible in a capture).

docs/formats is not touched (no data format).

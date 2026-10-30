# Design

The frame keeps the original's scratch colour as `Frame::clear` and its
clear colour as `Frame::background`; `Frame::sky_colour` mirrors the
scratch. The scratch starts at opaque black because the constructor sets
the alpha slots of the scratch and of the flash product to 1. Without fog
and without a sky the scratch is left as the last frame wrote it.

The rain effect and sound are modelled as state (`RainEffect`,
`RainSound`); a request creates each once. The sound's start event fires
once (the model sets its own started flag, where the original waits for the
audio system to set it).

The live system keeps the frame last published and writes Bevy's resources
only when it changes, so a still camera with no weather costs one compare a
frame. The clear colour it writes is the frame's background, which under a
sky is the level's haze, as at load.

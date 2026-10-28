# Design

The original has two colour flags. The linear-light setting (on in the
shipped game) gates the setters' linearisation and the blend's
post-linearisation of fill, fog and `p`. A second, zero-initialised flag
that no code writes gates the start-up linearisation of the built-in track
and, inverted, the frame's horizon linearisation. Exarust keeps the first
as `SceneEnvironment::linear` and the second as the constant
`TRACK_LINEARISED_AT_STARTUP`, so both paths stay modelled and tested while
the scene reproduces the shipped values.

Under a sky the sun placement copies the blended record's sun colour onto
the light node without conversion, so `environment` hands it to Bevy as a
linear colour; without a sky the level's sRGB sun colour is linearised as
before.

The frame's tint multiplier is the tint's own fourth slot: the constructor
writes sixteen bytes there and the frame scales the colour by the fourth
float, so no separate strength is kept.

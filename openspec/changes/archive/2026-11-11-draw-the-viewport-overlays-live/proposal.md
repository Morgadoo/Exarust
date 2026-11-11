## Why

PLAN Phase 3 (interface). The 3D viewport (T3DViewport) had its draw slot's
rules modelled with no live caller, and Exarust's frame-rate readout was its
own (a half-second mean in the 24-point base font at a fixed margin, shown at
the title too, set once at start-up). The original's draw slot ends with two
overlay lines whose rules and flags are in the assembly, and the version line
it shows at the title (retail main-menu reference: "V 0.9.5.2g" bottom left,
no frame rate) was missing.

## Evidence

REA (Exanima.exe), read in the assembly:
- The viewport draw slot sets its refresh flag unconditionally after the edge
  test, so the progressive still never runs in 0.9.5 (already recorded); the
  readout's counter runs only while the frame-rate flag is set and the title
  flag is clear, else its text is cleared.
- The readout: more than 250 ms since the last reading reads frames over
  elapsed seconds (double), formatted by the engine's float formatter with two
  decimals and trailing zeros, plus " FPS"; x is the inner right (width less
  the border, from the form layout) less the border, the unit and the main
  font's gap per byte, the baseline the line index (1) units plus the font's
  line height; a black alpha-0x80 shadow a pixel up and left, then the skin's
  seventh label ink with its alpha replaced by 0xa0.
- The version line: x the unit, baseline the form's inner bottom less the
  unit; the same shadow, the ink as it is.
- The flags: the viewport constructor clears the frame-rate flag; the ShowFPS
  setter writes it at once; every session start (arena, story, practice and
  both continues) sets it from ShowFPS and clears the title flag and the
  version flag; the session's leave clears it; the return to the title and the
  splash's hand-over set the title flag and the version flag.
- The camera tick eases a height offset toward a camera-node field at the zoom
  spring's rate; no store writes that field and no routine is handed its
  address outside the tick, so it stays zero in 0.9.5 (documented, not ported).

## What Changes

- exa_game: `view_overlay` (ViewOverlays flags, FpsReadout, the readout and
  version places, one overlay node rasterised by the GUI's rasteriser into the
  line's own box when it changes); the flow's title return and session start
  and the splash's hand-over set the flags.
- exa_ui: the Bevy-UI readout is gone (`fps` keeps the ShowFPS resource);
  `viewport::fps_text` uses the engine's float formatting.
- docs/UI.md (Frame rate), docs/PLAYER.md (free camera height offset), crate
  READMEs.

## Out of scope

The session leave's clear of the frame-rate flag on the short return to the
menu form (Exarust's every other leave goes to the title or a new session);
the level font atlases (the base font's glyphs stand in, as for all GUI text).

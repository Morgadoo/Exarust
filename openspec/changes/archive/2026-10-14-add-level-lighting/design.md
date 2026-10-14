# Design

## Decisions

- **Presets win over the record.** The game picks the lighting by level name
  (indoor preset for story levels and the indoor arenas, outdoor weather for
  `arenao*`). Exarust does the same and uses the record's sun mode only for
  other levels.
- **Ambient relative to the typical level.** The original's ambient scale
  depends on its shader. The record's colour is applied relative to the common
  0.6275 grey, so most levels keep the calibrated look and warm or cool levels
  are tinted.
- **Sun calibration.** Intensity 4.5 (with sky) × 160 lux per unit, matched by
  eye against the viewer's exposure. Two shadow cascades out to 60 m keep
  software rendering usable.
- **Fog** is exponential with the outdoor preset's ~1 km distance. The
  original's formula is in shader files that have not been decoded.

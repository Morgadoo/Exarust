## Why

Most of Exanima's levels are inhabited by skeletons, ghouls, golems and
other races, and Exarust only showed the humans: 374 of the 477 placed
characters were skipped.

## What Changes

- Decode `races.rdb`: each race's name, body model stem and movesets. A
  character's race byte maps to the race with id race + 1.
- Spawn every placed character in its race's body (`<stem>base.rfc`)
  playing its race's moveset. Bodies of the human frame (human, skeleton,
  ancient) are physical and dressed; the others are animated only.
- Skin every race's body from its seams. Only the human body seeds the
  weights apparel takes.
- Scale clip root positions by the body's leg length relative to the human
  body (`Stature`), so taller races stand on the floor.
- Viewer: `--race <n>` shows a race's body in the character view.
  `exa-tool corpus levels` counts placed characters by race.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `level-decoding`: placed characters of every race.
- `motion-playback`: clips fit bodies of other statures.

## Impact

- New `exa_formats::races`; `exa_assets::model`, `exa_anim`, the viewer,
  `exa_tool` corpus; `docs/formats/OTHER.md` and `RFC.md`.

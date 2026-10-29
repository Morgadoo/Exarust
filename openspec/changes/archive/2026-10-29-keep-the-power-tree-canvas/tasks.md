# Tasks

- [x] Split the frame's draw out of the draw system (`frame_draw`) and share the scripted-key test app
- [x] Walk each triangle row over a conservative span; test it against the whole-box rasteriser
- [x] Keep the canvas, redraw only the changed batches' boxes, in row bands on the compute pool; test byte identity on synthetic and real draws
- [x] Hand the canvas over with straight alpha at GlobalZIndex 910, re-uploading only when redrawn
- [x] Update the exa_game README and docs/THAUMATURGY.md (docs/PLAN.md is updated at integration)
- [x] fmt, clippy -D warnings and tests for exa_game

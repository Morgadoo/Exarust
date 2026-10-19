# Design

## Draw

`submit_shell_index` starts from the section spans. It counts the room, and when the flag is set it asks `shell_index_upload` for the header, the payload and the buffer targets. The mesh stores one index per triangle of that payload. `shell_index_unmap` is true only for a name above 0 on that upload. A clear flag keeps the room, stores no upload, and does not unmap.

## Open questions

- The Bevy render graph does not issue the original's GL calls.

## Docs

`docs/formats/OTHER.md` gains the play submission. `docs/PLAN.md` and `docs/COMPONENT-MAP.md` rate the shell mesh at 75%.

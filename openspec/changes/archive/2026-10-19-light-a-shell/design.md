# Design

## Light

`shell_light` reads the state and the shown value. Opening compares the shown value as a double with 0.666. Above that, the raw level is one half. At or under it, the raw level is 0.75 minus the absolute gap from 0.75 to the shown value times 1.5. Any other state takes half the shown value. The returned level is one tenth of that raw level, through a double. While closing, the gain is the shown value times 2.8, through a double. Any other state keeps the gain it was given.

## Open questions

- The second light's two channels, and uploading the GPU buffers, are not in.

## Docs

`docs/formats/OTHER.md` gains the shell light. `docs/PLAN.md` and `docs/COMPONENT-MAP.md` rate the shell mesh at 65%.

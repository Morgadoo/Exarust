# Design

## Fade

`shell_size` is 800. `step_shell_fade` reads the elapsed time as the clock minus the stamp. Opening divides that by the size times 1.5. A result above 1 becomes 1 and the state leaves opening, or enters the hold state when held. The hold state moves to closing and sets the stamp from the size times one minus the shown value, rounded to the nearest even integer. Closing stores 1 minus the elapsed time divided by the size. A result that is not above 0 becomes 0 and finishes. A held close stores 3 times that result, then shows 1. Any other state is unchanged.

## Open questions

- The shell light, and uploading the GPU buffers, are not in.

## Docs

`docs/formats/OTHER.md` gains the stored size and the fade. `docs/PLAN.md` and `docs/COMPONENT-MAP.md` rate the shell mesh at 60%.

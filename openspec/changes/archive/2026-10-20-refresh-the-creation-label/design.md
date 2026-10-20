# Design

## Behaviour

The clock is the global at `0x1004997c0` and the stamp the global at `0x1004f0420`. A difference under 300 ends the method without touching anything. Otherwise the stamp takes the clock.

When the widget at `0x198` is null, a label is built from the object's `0x1b0` and its `0x190 + 1`, then stored. The draw then runs through `0x1000cd390` with the widget. The finalizer runs either way.

## Model

`refresh_creation_label(clock, last, has_widget)` returns whether the refresh ran, whether it built the label, whether it drew and the new stamp. The widget is assumed to exist after a successful build.

## Open questions

The label build and draw bodies are not in.

## Docs

This method is 271 bytes. `SG_Character_Create` moves from 13% to about 14%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

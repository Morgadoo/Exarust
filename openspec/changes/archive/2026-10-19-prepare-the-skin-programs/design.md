# Design

## Behaviour

When the guard is already set the method returns no programs. Otherwise it builds three:

1. Kind 5 with the solid source `FragColor = vrCol;`. It gets `MPrj` and sets it to 2.
2. Kind 0x15 with the textured source that multiplies by `TexCol` and `Tex`. It sets `MPrj` to 2 and gets `Scale`.
3. Kind 5 with the rounded source that shapes alpha from `Scale`. It sets `MPrj` to 2, sets `Tex` to 0 and gets `TexCol`.

Each "get" is a lookup with no value; each "set" carries the value. Three `MPrj` sets, one `Tex` set and two lookups make six uniform operations. The exact GLSL text is kept with each program.

## Open questions

The shader-kind constants 5 and 0x15 are named only by number. The GPU calls behind the build are not in.

## Docs

This method is 333 bytes. `TGUIDrawSkin` is 48%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

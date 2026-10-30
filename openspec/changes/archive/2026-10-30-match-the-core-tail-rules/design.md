# Design

The terrain tile builder now keeps the original's byte channels as the
source of the covers. Each leaf of the rule walk adds `ROUND(w·255)`
(CVTSS2SI to 64 bits) to its material's byte and compares the low 32 bits
as a signed integer with 255, so a positive weight saturates, a NaN weight
adds the integer indefinite's zero low word and a negative one wraps. The
summed leaf weights pick the used materials, the merge turns the middle
channels into their share of the running byte sum, and the merged bytes
are the splat texels. The shader already draws layer 0 opaque, so the
first channel's raw byte is harmless. Over sixteen materials the tile
keeps the heaviest fifteen and material 0.

The rule walk is the original's explicit stack: an ordered input at or
below the low bound follows the below child with the whole weight, one at
or above the high bound the above child, and anything else splits (above
child first). Only the walks' starts (each painted layer, then the
remainder) are gated by the double 0.001.

Duplicates removed: the object manager reuses `Item::mechanism` for its
collision body and `scene_tree` (now over the parsed volume section) for
model-less records; the world-load walk reads `Container` and appends
through `Fill::add_slot`, and reads the palette byte through
`Quantised::count`/`Quantised::signed`; the volume query and thaumaturgy
use the shared reciprocal square root. `exa_formats` keeps its own mirrors
because `exa_physics` depends on it.

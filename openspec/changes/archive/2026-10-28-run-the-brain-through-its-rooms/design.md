# Design

## Sight

The original asks the world's sight method for each candidate of the
visible and nearby lists. A grid level installs the room reach test, which
the world crate already ports for lights, so the AI reuses it with the
room tracker's door state. A position in no room is taken as seen: the
original's test would refuse it, but there every character stands in a
room, and a placement on a cell the flood left out would otherwise blind a
brain for good. The test is a few table reads per pair, so the frame cost
stays flat.

## Once a body tick

`BrainClock` remembers the body tick each pass last ran on. Bevy's frames
can outnumber the fixed ticks; the original calls these passes from the
body tick. The flight walk steers on the tick and puts back its last
velocity on the frames between, because the duellist clears every NPC's
desired velocity each frame.

## The look chain

`look_chain` composes the existing `idle_look`, `look_smoothing` and
`smooth_look`. The gait helper's write goes to the body's heading
direction, kept on the brain as `Gaze::body_heading`; the running game has
no head node, so the model still turns with the eased look and a fleeing
brain is turned by its flight.

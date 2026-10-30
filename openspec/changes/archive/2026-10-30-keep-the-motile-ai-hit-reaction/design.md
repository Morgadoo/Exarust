# Design

The threat estimate takes the observer's nearby bodies as ids and aim
points supplied by the caller rather than querying the scene, since the
body's aim point is not an Exarust component. The squared distance and the
dot follow the listing's addition order (`z² + (y² + x²)`); every gate is a
`COMISS` with a parity jump, so a NaN distance or dot passes the gates and a
NaN share never becomes the best.

The hit reaction's cap gate jumps out on parity, so a NaN concern gets no
rise; the floor and the final cap jump past their store on parity, so a NaN
rise produces a NaN concern written through `Contact::set_concern`, the
contact's concern setter the original also calls.

The grabbed-flag step returns what to do (`Grab { held, released }`); the
release itself is the body's grip call, which Exarust lacks.

The alert look's foe branch measures its 45° gap from the AI's look yaw:
the same field is overwritten with the foe's yaw and is what the turn's
heading copy reads, so the lane's switch to the physical facing was wrong.

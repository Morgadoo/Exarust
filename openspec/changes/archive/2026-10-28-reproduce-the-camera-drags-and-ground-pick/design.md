# Design

The original's handlers mutate one camera controller and receive GUI mouse
events; Exarust has no editor viewport, so each routine is a pure function
over the controller's fields as arguments, returning what the original
writes. Only the stepped wheel zoom has a live caller (`chase_camera`),
since the chase camera's zoom stops are exactly the stepped camera's six.

Units: `nav_pick` works in Exarust's metres (the original's centimetres
over 100: the 4000 cm reach is 40 m and the ground grow values 5.0 and
0.1 are 0.05 m and 0.001 m); `nav_camera` keeps the original's own units
for the preset and zoom fields, as before.

The wheel's 120 units a notch come from the original's stepped routine
(`units · 1/120` levels); a Bevy line is one notch. A pixel scroll delta
has no original counterpart; Exarust reads 24 pixels a notch (a labelled
guess) so a touchpad cannot jump the zoom to an end stop in one frame.

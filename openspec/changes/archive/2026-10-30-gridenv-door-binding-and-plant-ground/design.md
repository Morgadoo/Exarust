# Design

## Door binding

The original converts the door's position to a cell with the floor helper,
looks the cell's room up, steps one cell opposite its third matrix row's
`(x, z)` and searches the room's links for `(cell, behind)`; with a body it
writes the body into that link and copies it into the other room's
`(behind, cell)` link. `DoorBodies` keys both directions on the cell pair,
so either side reads the same body. It is a resource-free rule: nothing in
Exarust places door mechanisms yet.

## Quick yaw

The engine's yaw helper is `sign(x)·acos(z / √(z² + x²))` with a polynomial
arccosine (0.0535x² + π/2 − 0.2075x, times √(1 − x)), not an exact
`atan2`. The quadrant edges compare it, widened to double, with the double
π/4 and 3π/4 and with the f32 π/2 widened then offset by the double π/4.
The polynomial's error moves the diagonal (1, 1) to 0.78517902, inside the
first quadrant. The helper moved to `exa_anim` so both `exa_world` and
`exa_ai` use one copy.

## Plant ground

The baker passes the point and a position whose height it never sets (the
previous plant's result is still in the slot). With 20 cm cells the
correction is a plane solve that cancels that height, so Exarust passes 0.
The diagonal test does not separate the two planes it solves
(`1 − fx + 2·fz > 1`); the port keeps the original's choice. The result
is the instance height.

# Design

## Table

`Bindings` stores the controller-mode flag and `[u32; 23]` virtual-key
codes. The default array is the shipped table: `0x01`, `0x57`, `0x41`,
`0x53`, `0x44`, `0x58`, `0x10`, `0x12`, `0x04`, `0x56`, `0x11`, `0x49`,
`0x09`, `0x4B`, `0x52`, `0x46`, `0x4A`, `0x54`, `0x43`, `0x50`, `0x51`,
`0x27`, `0x25`.

VK 1, 2 and 4 map to the left, right and middle mouse buttons; the table's
keyboard slots map to Bevy `KeyCode`s. Shift, alt and control are generic
Windows virtual keys, so both sides count as held (`0x10`, `0x11`, `0x12`).
The table is a Bevy resource initialised by `PlayerPlugin`; the movement
request, combat footwork, attack request, combat toggle, orbit drag and
cursor interact all read it. The right mouse button is not in the table
(the original reads VK 2 directly for the walk-to-cursor request) and stays
fixed.

## File

`encode` writes little-endian: `101`, `controller as u32`, `23`, then the
codes. `decode` reads the version, then the next word: if it is 23, the file
is the older shape without a controller flag and that word is the count;
otherwise it is the controller flag and the next word must be 23. A wrong
version, count or a truncated file returns `None`. The old shape's
controller mode is false.

## Cursor Look

`CursorLook` holds `active` and the saved pointer. On the first frame the
bound slot is held, the system sets `active`, saves the pointer, sets
`CursorGrabMode::Locked`, hides the pointer and moves it to the window
centre. Each frame the horizontal mouse delta turns the player's facing,
heading and combat aim by `3.6 · dx / width`, normalised to `[-π, π]`; the
camera target yaw is set behind the facing (`yaw + π`). The UI's
`CursorCapture` hides the world cursor and keeps the system pointer hidden.
On release (or focus loss) the grab is cleared, the pointer is made visible
and moved back to the saved position.

With the pointer locked, the OS keeps it centred; the one explicit centring
on capture also covers backends that do not lock. The turn is applied to the
body directly, so the head and the combat aim follow immediately; the
fixed-step steering keeps movement along the new facing. `player_combat`
returns early while captured, so the cursor's ground point cannot overwrite
the turned aim.

## Tests

`CursorLook::turn` asserts 3.6 rad across a 1600 px window, 1.8 at half and
the sign; the binding tests assert the 23 shipped codes, the file round
trip, the old no-flag shape and either-side shift/alt/control.

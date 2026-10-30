# Design

The viewport's handler list is modelled as flags (`Handlers`): a preset
adds handlers to the original's list without removing any, so `install`
ORs the press and move flags and only replaces the wheel slot; a handler
present twice in the original's list cannot be told apart. The saved wheel
slot is a flag whose `false` is the empty slot, which the next preset
fills again from the viewport, as the original's null test does.

The mesh callbacks of the ground and kinesis picks stay behind `RayMesh`:
its nearest hit within the asked length is the distance the original's
callback writes back, and its face is the face index the callback returns
less one. The swept sphere radius the callbacks read is not modelled.

`gamepad_zoom` is a thin wrapper over the live `gamepad::zoom_step`; the
running game keeps its own per-frame pitch clamp, so the wrapper adds no
caller.

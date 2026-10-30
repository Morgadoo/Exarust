# Design

The fixes keep the lane's structure and correct the numbers and the
order. Where the lane added a second copy of an existing helper it now
calls the existing one: the rain's turn is
`engine_math::basis_from_pitch_yaw` scaled by `engine_math::scale`, the
Long Door's facing `engine_math::normalize_approx`, and the passes walk
`exa_render::subframe::Subframes` and its `multiply`. The Energy unit's
creator rule is a function beside the power class the cast already
pushes, not a parallel struct.

The rain pass's matrix is row-major like the renderer's: the scaled
turn's rows with `w = 0`, the drop box's centre with `w = 1`, times the
sub-frame's composite. `EyeVec` subtracts the centre before dotting with
each scaled row, summing `(x·r.x + y·r.y) + z·r.z` as the helper's packed
adds do.

The vanish table stays data on the vanish effect; its frame callback is
`advance`, its phased-body hook `phased_body_done`. Neither is driven
live, since the casts run the vanish through the power classes' phases.

The Barrier's GI frame takes the engine's look basis of its facing (a
labelled guess for the node's basis), the drawn centre shared with the
frame through `Barrier::drawn_centre`, named as the parallel lane names
it so the two merge as one helper.

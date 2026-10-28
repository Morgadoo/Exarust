# Design

The compass is painted on the CPU into a canvas covering the turned
square's axis-aligned box and shown as a UI image; it is repainted only
when the window size, GUI unit, camera axis or opacity changes (each frame
the camera turns, about 96–270 pixels square). Sampling is bilinear with
texel centres, as the original's linear filter. The camera axis is the
chase camera's `Transform::back()`, which is the camera frame's third row
the original reads. The approximate reciprocal root comes from
`exa_physics::engine_math::reciprocal_sqrt`, so the square is about 0.02 %
smaller than exact, as in the original.

The level curtain the original draws is the death fade `exa_ui::fade`
already wires; `game_gui::LevelCurtain` stays the tested state rule.

The form and counter rules are pure functions returning plans or
rectangles; the live screens do not call the span or set-controls plans
yet.

# Design

## Cause

`sim_control::sim_keys` toggled `Time<Virtual>`'s pause on `KeyCode::KeyP`,
the shipped binding of the power tree (virtual-key 0x50). The tree screen
stamps its fades with `Time` in `Update` (the virtual clock): showing sets
the opening stamp 1000 ms ahead, and `present_window` computes the window
alpha from the milliseconds left. With the clock paused in the same frame,
the time left stayed 1000 ms, the window alpha stayed 0, and every batch
the canvas drew (backdrop, nodes, bar, wheel) was transparent although the
canvas node itself was shown.

## Decision

Fix the debug path, not the screen: the pause moves to F8 (no default
binding, no other debug tool uses it), and the simulation keys are
suppressed while the player's bindings (or the shipped table when none is
loaded) bind them, so a rebinding cannot reintroduce the clash. The screen
keeps the game clock; whether the original's GUI clock keeps running while
the game is paused is not changed here.

## Verification

`power_screen::tests::a_scripted_tree_key_opens_and_shows_the_screen`
fails on the old key (the virtual clock is paused after the press) and
passes with the fix. On the GPU:
`exarust --no-gi --no-supersample --size 1280x720 --frames 300 --powers 0x201 --press P@200 --screenshot out.png play arena`
shows the darkened backdrop, the mind tree, the bar along the top and the
domain wheel at the lower left.

# Design

The end-of-match rules live in a new `arena_session` module next to
`arena_match`, reusing its round counter (`MatchRounds::finish_round`), its
Continue (`continue_match`), its fall rules (`fatal`, `resolve_fall`,
`log_fall`), its health word and its return booking (`return_to_hub`,
`count_returned_match`) rather than restating them. Each rule returns a
plain value naming what the scene must do (bodies to stand or generate,
the screen to open, the placement to save), because the running game has
no scene body generator for arena fighters yet.

The launch keeps the process restart the hub already uses. Only values
that the play-mode spawner can use cross it: the template, the event's
rank and tier for the layout, and one weapon object id per generated
opponent in layout order, 0 meaning the default weapon, so that a missing
variant never shifts the later ones.

Values the original reads from the GUI scale (the gauge geometry and the
preview camera distances) take the unit as a parameter instead of fixing
it at the base scale of 4.

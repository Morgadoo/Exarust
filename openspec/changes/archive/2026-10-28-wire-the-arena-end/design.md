# Design

## End screen

`ArenaEndScreen` mirrors the flow's end menu: `open_end_screen` shows it when
the match tick sets a transition screen, `update_end_screen` reads Enter, the
arrows and a pointer release over the row into an `ArenaContinue` message,
and `draw_end_screen` paints `end_menu_screen::end_menu_frame` through the
GUI layers. `continue_round` acts only on that message (or the unattended
auto-continue), so no other key continues.

## Bracket

`bracket_next_fight` clears the losing bracket with team one forced (the
player's side won the watched fight). `raise_fresh_match` walks the slots and
stops at the event's side count, exactly as the original's team walk, and
derives the role flag from the teams it formed; the live scene uses that flag
for every raised opponent.

## Generated records

`generated_record` runs the NPC pass, the outfit pass, then writes the
record: look fields, skill rows, personality, the event weapon in the main
hand, the outfit's hands and worn pieces. All draws come from one `Lcg`, the
arena roll; the original reseeds from the clock before each body, which is
not reproducible, so the roll continuing is the deterministic stand-in.

## Launch and return

After the rebase onto the in-process level change the career stays in its
resource and the launch's setup (now with the weapon row) in
`PendingArenaLaunch`; the end and the forfeit both settle into the career and
send `ArenaExit::Hub`.

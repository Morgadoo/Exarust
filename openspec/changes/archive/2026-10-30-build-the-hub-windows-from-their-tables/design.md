# Design

The Settings form's route (records laid out at unit 4, the form and its
records rescaled, the pointer through `gui_controls::dispatch`) is made
general in `exa_ui::record_form::RecordForm`, which keeps one record list,
plain child panes (Manage Company's two tables) and the coin controls'
amounts. `exa_ui::creation_forms::FormRecord` gains the words at 0x38 (as an
amount), 0x48 and 0x50.

The hub keeps its primary window (board, Manage Company, trade window) on
the window manager's task and stacks the table-built windows over it in
`HubWindows::forms`; the top form takes the pointer, and a world click made
by a press or release on a form is not the hub's. Callbacks map onto the
existing rules: `manage::ArsenalForm` (rows, steps, slider, shelves,
dismissal), `career::Company::end_day`, `tournament::forfeit_tourney`.

Guesses and gaps: the stacked windows draw without their show fade; the
original commits a loadout step after the frame, the hub at once; the
kit's point cost is written as 0 (the hub's career has no item points);
Arsenal, Recruit and HIRE open forms the hub does not build.

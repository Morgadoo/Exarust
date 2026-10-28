# Design

The hub's rules stay in `arena_hub`, `arena_spawn`, `arena_board`, `manage`,
`shop_window` and `arena_match`; the new modules only call them.

- One plugin, `ArenaHubPlugin`, owns every resource and message its systems
  read (`ArenaHub`, `HubRoll`, `HubWindows`, `ArenaCareer`, `HubItemNames`,
  `LastHubClick`, `WorldClick`, `ArenaMatchLaunch`, the input resources and
  `GuiFrames`), so `play exanima01` runs it without panicking; outside a hub
  level its systems return at once.
- The click goes through the hand's existing release path: a release that
  did not become a drag picks with the hand's own ray and sends
  `WorldClick`. An open hub window takes the click first (its own hit test,
  or a press outside closes the modal form), so the hook's confirmation
  gate is the open-window state.
- The busy flag follows the original's writes: the board's open and Manage
  Company's open set it, their closes clear it; the shop leaves it alone.
- The scene name is the stored sixteen bytes: Exarust's reader turns each
  byte into one character, so the hook turns the characters back.
- A match launch is a message the launcher turns into a restart with
  `--arena-match`, reusing the viewer's fighter spawning; the career stays
  in the hub process and no return is wired.

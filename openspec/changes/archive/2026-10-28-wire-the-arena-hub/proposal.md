# Proposal

## Why

docs/PLAN.md, Phase 7 (game modes and content), arena paragraphs: the
arena's match, staff, report, save-chunk and company-window rules were in,
and "a hub screen to draw and wire the windows and actor spawning for
matches come next". Only the hub's music draw ran in the game; its entry,
click hook, fall hook, the match board, Manage Company, the trade window and
a match launch from the hub were tested rules that nothing called.

## What Changes

- `exa_game::arena_hub_live` (`ArenaHubPlugin`): the hub entry runs once
  when the `arenahub` level is in (arena mode, or the play mode that loads
  it), draws the music, installs the hooks, keeps the busy flag and places
  the manager; the session hook runs every frame and ends the session (and
  wipes the slot through the existing slot code when the host supplied it).
- `hand::WorldClick`: a left click that is not a drag carries its pick to
  the hub's click hook, which opens the board (`arena_board::open`), Manage
  Company (`manage_refresh`) or the trade window on a `GuiTask`
  (`exa_game::hub_windows`), drawn through the GUI draw path.
- A board Enter, Begin or Continue plans the match (`start_match`, the
  layout, `generic_plan`, `hire_plan`, `beast_plan`, `player_participant`,
  `pick_weapon_slot`) and the launcher restarts on the drawn arena with
  `--arena-match`.
- Review fixes: opening the board makes the hub busy, as the original's
  board open does (its close clears it); the node name is compared as the
  stored bytes (the library keeps the leading 0xAA, and the scene reader
  keeps each byte as one character), not with a byte prepended; the
  play-mode hub roll is the byte-reversed counter, as before.

## Evidence

The retail hub click hook, the board's open and close routines, the Manage
Company close and the hub entry, read from the assembly (busy flag writes
at the board open, board close, manage open and manage close; the click
hook's 16-byte compare and the ordered side compare that sends NaN to
Manage Company). The library entries `shop_arena`, `noticead` and
`noticeboard a` in `Objlib.rpk` hold the nodes `\xaaArenaShop` and
`\xaaArenaMach`, with no rigid-body chunk.

## Out of scope

- The pick of static scenery: the hub's notice boards and shop counter have
  no body, and the static colliders are built in batches that keep no node,
  so a click on them picks the level in the running game.
- Which node field the ArenaMach side value is (the node's world x stands
  in), the shop's first stock build (needs a live `ShopWorld`), the
  merchant's own stock, the session end screen, the match's return to the
  hub, the windows' own close controls and the board's pictures and items.

## Phase

Phase 7 (game modes and content), the arena: the hub screen and its wiring.

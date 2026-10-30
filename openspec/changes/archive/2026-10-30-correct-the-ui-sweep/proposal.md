# Proposal

## Why

A sweep over sixteen small UI components (docs/PLAN.md Phase 7, game
modes and content: the menus, the character creator and the GUI forms)
read their listed routines against the assembly. Most rules were already
present and matched. Three rules were wrong: the creation pickers
measured the whole form instead of its client, the compact skill tree
narrowed a pip's v range that the original keeps and halved a node's
alpha before the fade scaled it, and a first pass at the main menu read
the end pages' forfeit from the game mode instead of the session state.

Evidence: the original's assembly for the picker base's pick, marker,
tone quads, hair cells and shade, the form layout that sets the client
rectangle from the border, the form and skin constructors (border 2, a
frame stretch of 2), the compact tree's pip and node painters and the
main menu's click handler (REA and the objdump listing).

## What Changes

- `exa_ui::pickers::field_size` (wired through `exa_game::creator_screen`):
  a picker's client is its form size less the border of 2 on every side;
  the creator picks from and draws the physique cross, tone quads, hair
  cells, shade and markers at the client's corner.
- `exa_ui::compact_skill::lay_out_pips`: the narrowed last pip keeps the
  v range; only the u range shrinks.
- `exa_ui::compact_skill::technique_nodes`: the slot in training halves
  the line's alpha byte after the fade has scaled it; `NodeDraw::colour`
  is the drawn RGB under that alpha.
- `exa_game::menu_pages::menu_command` (wired): the end pages' third row
  forfeits unless the session state has bit 4, as before the sweep.

## Out of scope

- The marker's skin primitive (Exarust's square stays a labelled guess).
- Mounting the compact tree, the tooltip form, the colour swatch and the
  textured skin frame in the running game; the doll's mesh pick for a
  drag from the figure.
- The hair list icons and the transition screen's close-time engine
  words.

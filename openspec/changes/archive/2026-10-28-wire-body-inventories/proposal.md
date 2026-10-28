# Proposal

## Why

docs/PLAN.md, Phase 4 (combat and items), the gamegui2 and itemgui2
paragraphs: double-clicking a dead body opened its loot window, but its
pack and equipment panels were empty, the window's flag word was passed as
0, the deferred close was a tested rule nothing called and the player's
carried list was not bound to the inventory's pack.

## What Changes

- `exa_game::body_inventory`: a body's pack loads its record's carried list
  through the item panel's character-list load (`Container::load_list`, the
  saved cells in sixteenths of a GUI unit, the model's bulk byte), and its
  equipment panel lists the worn items and the shields of the set in the
  hands through the worn/scatter pass. `take`, `give` and `transfer` move
  possession entries; a load-time pass gives a character with a record its
  `Possessions` when no other path did.
- `exa_game::panel_live`: the live left drag between open panels through
  the item panel's press, accept, drop and drag-end rules; a press over an
  open screen is kept from the world.
- `exa_game::gui_screens`: the inventory binds the player's carried list,
  a loot window binds its body's panels, the show rule reads the body's
  profession word, and the deferred close closes a window whose body is no
  longer dead.
- `exa_items`: `Possessions.worn` keeps the record's id-and-flags pairs;
  `ItemCatalogue::bulk` reads the model section's `0x24` byte.
- Review fixes: a hand item is listed only when it has a shield section
  (held weapons were listed); the show rule reads the record's profession
  word, not the condition word; the deferred close's state mark reads 0
  instead of the condition word; the scatter's seed is labelled a stand-in
  for the engine's shared generator.

## Evidence

The retail loot window's bind, the item panel's character-list load and the
equipment load, read from the assembly: the hand loop over the two pairs of
the set in the hands calls the object-database section lookup with the
shield tag (`0x8000`) and skips the item when it returns nothing; the run-
time seed `0xba28` is stored before the pass while the draws use the engine
generator. The show rule masks the record's word that the character-creation
and AI units read as the profession (byte 1 the race). The deferred close
compares the body's condition with `0xa00`.

## Out of scope

The off-screen item renderer (the reserved icon slots draw nothing), the
character view and drops onto the equipment panel, re-dressing a body
after a worn item is taken, writing possessions back into the record and
saving a body's pack, the GUI-to-world drop, the level-load promotion of
level-local item records (their bulk counts 0 and they are not known as
shields), the player's starting gear.

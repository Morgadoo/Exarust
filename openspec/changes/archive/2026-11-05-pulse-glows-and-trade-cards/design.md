# Design

## Glow pulse

The pulse task list is the game unit's global ordered list; each frame a
task computes its level from the game clock and writes it. Exarust keeps the
list in `exa_script::objects::Objects` and the placed objects it drives in a
registry (`GlowPulses`), the task's object number being the registry slot.
The original writes the level as the geometry's per-draw emissive scale with
its rescale bit; Exarust's materials carry the emissive colour, so each part
takes its own material copy and its loaded emissive is kept beside it, then
scaled. A node's type word is not kept by the model reader: a node with mesh
parts reads as type 9 and a voxel light node as 0x41.

## Crystal drain

The socket drain reads the item's weapon section and, without one, its
apparel body, each at its first socket word (kind byte, charge byte). The
original's section pointers include the 8-byte section header, so its
offsets are the body's plus 8.

## Trade window cards and the layer order

The trade window's panels are item containers; the card request from a
panel passes the entry's flags, and the stock is loaded with `0x80000000`,
which also makes the card show the item's whole value. The card form's
attribute word has the upper-layer bit; the trade window's (the form
default with `0x800000`) does not, so a raised trade window stays under the
card: the card draws after the hub windows.

## Open questions

- The scripted object's script run: the script record and its interpreter
  setup are traced; the live interpreter call is left for a later change.
- Who files container crystal sources: answered by tracing the callers of
  the drain's add routine (REA), later.
- Format page: `docs/formats/OTHER.md` (object database, socket words); no
  format change here.

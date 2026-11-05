# Design

Formats: `docs/formats/OTHER.md` (weapon stats; the ability word and the
socket words of the weapon section).

## What the original does

### The install and the list

- When the hands are dressed, each held item whose weapon section's ability
  word is 1, 2 or 3 registers its check on the body's item-effect list (one
  entry per item). Type 2 also sets the item's hit hook. Type 4 ("Sceptre
  A") registers nothing.
- The body tick runs the list once a game second. The list's counter
  advances on each run. A check whose item is no longer in a hand drops out.

### Type 1, the lightning mace ("UB5 FlangedMace", a factory mace)

- Check: the first socket's low five bits 1 with a charge below 0xfe, or
  0x11 with a charge below 0x7f. Charged with no live arcs: raise them.
  Spent with live arcs: end them (respawning off, ending on, hooks cleared).
- Raise: hit hook and end hook on the item; the hum arcs are a child node of
  the item at (0, 53, 0) cm:
  - 20 records of 5 points, size 2;
  - the first record timed `random(10) + 10`, every record respawning;
  - light 24, volume 0.075, `electricity03`, the segment ×0.75;
  - a voxel light (0.6, 0.6, 1), base 16, 180 cm, starting dark.
- Hit hook: the struck character's shock with strength 0 along the hit's
  direction, then (through the game's event queue) the socket's charge
  gains 10, or 0x80 for a kind with bit 0x10, while `charge < 0xff − step`;
  otherwise it is set to 0xff and the arcs end. A spark burst is left at
  the contact, in the struck body's frame.

### Type 2, the self-sharpening sword ("Swrd Self Sharp")

- Check, on a run whose counter has its low three bits set: slash byte
  (weapon byte 0x37) below 0x8c, first socket kind 2 (Power Crystal B),
  charge below 0xff → charge + 1, slash + 2. Then, on that run either way,
  W11 (the record's slash share) = `0.5·exp(0.7·ln(0.01·slash))`.
- Hit hook: slash above 0x40 loses 4, else becomes 0x3c; W11 follows.

### Type 3, the lit-blade byte

- Lighting runs the moveset choice with the Flaming overlay. That choice
  clears the body's lit-blade byte first. Then lighting raises the byte,
  stores −1 as the grip mode and sets the old mode again, which runs the
  grip's callback.
- The two-handed grip callback reads it: when the held item's ability word
  is 3 and the byte is set, a requested mode 2 is turned into 1 inside the
  callback. So a lit greatsword keeps both hands on its grip at rest.
- Putting out does not run the moveset choice: the overlay and the byte
  stay until the next one.

### The socket control

- Hover: socket `(x − x0) / 5u`, if it exists.
- Drop of an item payload: only crystals A, B, C, AR. The words are
  re-read from the item. An occupied socket refuses a drop from itself,
  else gives its crystal back to the player with its charge. The socket
  takes the kind (keeping its 0x80 bit) and the charge (flags bits 16–23),
  the commit callback runs, the source's drag ends.
- Give-back: into the open inventory's pack at its centre, else onto the
  player's carried list.
- Drag start: the payload is the crystal with its charge; the socket index
  is kept. Drag end (accepted): the socket's low byte becomes 0x80, the
  commit runs.
- Commit (the card's callback): the four words into the weapon section
  (byte 0x48), else the apparel body (byte 0x218); an AR crystal fires game
  function 4.

## Choices

- Exarust scans every character's two hand slots once a second instead of
  keeping a list; the counter is per character in the ability clock.
- The arcs are thaum `LiveArcs` with an `ItemArcs` rider that sets their
  anchor and frame from the item each fixed tick, before the arcs tick.
- The lightning hit's deferred work runs at once. The burst stays at the
  contact point.
- The lit-blade byte is the `BurningGrip` component; the held item's
  ability word is `HeldAbility`, set by the check.
- The socket drop and drag live in `panel_live`'s press and release, with
  a `Panel::Socket` drag source. The panels now read the scripted pointer,
  so `--drag-at` works on them.

## Open questions

- When the game's event queue runs its entries (next frame or later): read
  the queue's consumer with REA. Not needed for the rules.
- Whether the factory mace's look matches the shipped one: compare in the
  retail game.

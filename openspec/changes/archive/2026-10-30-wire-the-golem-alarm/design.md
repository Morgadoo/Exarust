# Design

Format page: `docs/formats/OTHER.md` (the `0x8000ac07` volume section).

The zone check runs in the AI manager's frame, after the rise, as the
original's does, only for the player (the only branch ported). The checked
point is a component on the player; it starts at the origin [guess: the
body is allocated zeroed]. The check uses the character's feet position in
centimetres, the same position the perception pass reads.

An alarm zone's box is the placed object's container volume: centred on
the object's origin, its half extents the section's three, its axes the
placement's rotation rows, its node radius the extents' length
[inferred: the object manager's container volume keeps only extents and a
radius]. The shipped zones stand on the floor (centre 100 cm up, half
height 100), so the feet position is inside them.

The comparisons keep the listing's: the radius test in single precision
lets a NaN through, the two box tests convert to double and also let a
NaN through (an unordered checked point counts as outside).

The radius query walks the room holding the zone's centre and the rooms
behind its exits (a room behind two exits twice), offering the characters
standing in each whose race word carries the AI bit and that are strictly
within 800 cm on x and z. Exarust has no per-room character lists, so a
character's room is read from its current position with the level's room
grid; without a grid every character is offered [guess].

The wake callback refuses a golem whose statue byte (`Npc::statue`) is
clear. An accepted golem is woken through `golem::wake` (the race word
loses its statue bit, the statue byte clears, the body is kept awake for
256 ticks, the home is taken, `golem_act` plays) unless its body rules
carry `0x100`; either way its contact list becomes the player alone at a
concern of 10 (the listing writes the count 1, zeroes the entry and stores
the player and the float 10 in the entry's concern word, which the ally
sense reads as the concern). The pitch draw comes from the engine
generator the sound bank keeps.

Open questions and how they were answered:

- Who calls the wake? REA callers: the alarm callback and the ally sense;
  the latter's branch is unreachable (statue byte set and clear).
- What is the alarm zone's number? The section's word at `0x10`, which the
  zone check also passes to the inventory search: an item id (the key
  tokens), confirmed by the search reading the record's carried list and
  nested container sections.
- What does the trigger write on a woken golem? The contact list (count,
  entry, character, concern), read against the ally sense's use of the
  same words.

Not answered: what sets the state-flag bit `0x20000` (it ends the check).

# Design

## Callers, as traced

- The use hook is a function pointer the controls install at start-up. The
  item panel's mouse handler calls it on a double click whose class is 2
  (after closing the card), with the item, its entry word, the slot from one
  and the panel. Exarust's double click on a panel entry writes `UseHook`.
- The cursor's callback is installed on the world view and holds the mouse
  for it. The view calls a pending callback with event 0x80 before it serves
  its next mouse event, and an answer with bit 0 set takes that event. So the
  cursor ends on the next press of any button, which nothing else sees.
- The dispatcher runs when the desk's word has bit 0. Its setter was not
  traced; the desk's own button bits read 1 left, 2 right, 4 middle, so
  Exarust takes it as the left button (a guess).
- The GUI drop is claimed while the drag is held over the world view; the
  task runs on a worker thread in the original and on the next frame here.
- The `GameEvent` handler is registered once at start-up; the unlock branch
  ORs bit 4 into the background word and writes the whole word, then
  flushes the store.
- The light toggle reads the first held item's part mark (the same mark the
  flip turns over at its end); the weapon attach sets that mark from the
  item's full flags while its job keeps only the top bit for the record.

## Open questions

- The desk word's bit 0: answered by a guess; a capture of the running game
  pressing the right button over a door with a key would settle it.
- Whether the flip runs for a torch (its link index): not traced; Exarust
  turns the part's mark at the toggle, so the next toggle is right either way.

## Frame cost

Every new system returns at its first test outside its case: the cursor's
end when the cursor is down, the claim without a panel drag, the drop run
with an empty queue, the lift without a marked body, the event consumer
without messages.

Formats: none changed (no docs/formats page).

## Order of a dress-first load

1. The dresser equips the item. The holder has no body yet, so the
   item's grip carries the placeholder key and a `PendingGrip`.
2. The body is built (`build_bodies`), from the skeleton's nodes only.
3. In the same frame the body's parts are drawn onto the nodes, and
   `register_props` builds the item's body at its grip under the hand.
4. On the first fixed tick, `resolve_pending_grips` fills the key, then
   `build_grips` ties the links and `mark_held` sets `held_by`.
5. On the third tick, the ground placement's Height Query leaves out the
   items whose `held_by` is the placed body's key.

The original builds a character whole and attaches its items to the built
body. Steps 2 and 3 restore that order for an item equipped first.

# Design

**Focus on demand.** The original asks for the Talk target inside the talk
routine, so Exarust asks only on the frame the Talk key is pressed (and no
dialogue is open), before the hotkey handler reads `DialogueFocus`. The
player's nearby list is rebuilt for that one query with the same rule the
brains use (`exa_ai::contacts::nearby` over the player's room and its
exits' rooms, `rooms::in_sight`), so nothing runs per frame.

**One projection.** The target query and the bubble placement call the
same view-then-viewport projection in the original; both use
`exa_script::bubbles::project` with a `Camera` built from the chase
camera's world transform (rigid inverse rows, eye in cm) and its clip
matrix's two scales, on the window's physical pixels.

**Panel state.** The engine's `Panel` stands for the original's panel list
and pick fields; `DialogueWindow` keeps only what the form adds: the place
made on the first talk, the eased height, the hover and the hold's end. The
form is shown while `Panel::open` is set; an emptied list keeps the box, as
the original's step empties the list without hiding the form. The engine's
`chosen` turning non-zero is the pick, which starts the 1000 ms hold.

**Paint.** One GUI frame per frame after `draw_screens`: the bubble boxes,
the bubble texts, the panel's box, and the panel's text in two batches (one
per ink). The system returns early while there is no bubble and the panel
is hidden.

**Pointer.** The panel's system runs after the screens' pointer tracking
and before the hand, ORs the pointer-over-panel into `PointerOverGui`, so
the world's press and click paths leave it, and sends `PickReply` on a
released left click over a band.

**Ids.** Exarust's player id (1) is a guess that level record 1 shares; the
cast mirror skips such a level character and the focus excludes it, so the
engine never takes it for the player. The companion takes the id after the
level's records [guess].

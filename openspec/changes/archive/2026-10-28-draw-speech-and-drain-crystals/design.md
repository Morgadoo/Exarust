# Design

The bubble and panel rules stay data-only: `exa_script::bubbles` holds the
list, its measure, placement and projection (the camera is supplied by the
app as a rigid inverse view and the projection's two scales), and
`exa_ui::speech` turns a frame into a rounded-fill batch and text layers.
`exa_ui` gains a dependency on `exa_script` for the bubble types.

The topic memory buffer keeps the original's shape (count word first) so
every index is the original's 1-based node number; a stored block is the
same vector, which makes the shifted load a plain slice copy.

The candidate record of the original holds six entries per speaker with no
bound check; a seventh line would overwrite the next speaker's record.
Exarust keeps the first six, a labelled guess at the defined behaviour.

The glow task and crystal walks are tested functions over `Charges` and
`GlowWrite` lists; nothing consumes the writes or calls the drain in the
running game yet.

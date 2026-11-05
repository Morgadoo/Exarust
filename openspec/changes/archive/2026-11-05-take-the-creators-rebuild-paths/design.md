# Design

The original picks the path by the control that fired; Exarust's creator
collects a frame's edits before the preview's regeneration rule lets it
rebuild, so the path is derived from the record's difference instead.
Each control writes only its own fields, so the two agree for single
edits; when one frame holds edits of both narrow paths both run, and any
whole-body field makes the whole rebuild, which covers the others.

The whole rebuild in the original rebuilds the same character object,
saving and restoring its bone frames. Exarust spawns a new figure entity
and gives it the old one's `Animator` and `PreviewTurn`, which keeps its
motion and its turn toward the pointer.

The hair-colour edit calls only the re-texture in the original, which
repaints the beard (the skin job takes the hair diffuse). The scalp hair's
material route is not traced; Exarust's hair parts carry the colour in
their own materials, so the hair is regrown with the new colour (labelled
in code).

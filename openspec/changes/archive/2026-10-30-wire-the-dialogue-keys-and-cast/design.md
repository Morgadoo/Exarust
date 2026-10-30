# Design

The original keeps one character object shared by the AI and the scripts.
Exarust keeps the AI's state on entities and the script side in
`exa_script::cast::Cast`; `mirror_cast` bridges the two once a frame. The
direction is per field: live fields (name, health, arousal, out-of-action)
flow world → cast; relations, topic memory and role values live only in the
cast; the role list flows world → cast unless a script changed it
(`roles_changed`), in which case the cast's list is written to `Roles`
first. Without the write-back, an `AddRole` from a script would be replaced
by the AI's older list on the next frame.

The common frame does no allocation: when the role ids match in order the
role merge is skipped.

`DialogueWindow::sync` compares the engine entries with the ones it built
from and relayouts only on change, so the panel's 500 ms height animation
is not restarted every frame.

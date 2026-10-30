# Design

The parts keep their existing signatures except `draw_panel`, which gains
the control's style word (the same word the shell takes) so it can draw the
style frames before its box. `selection_band` takes the highlight group
instead of two inks, because the original indexes the group by the
selected state itself. The raised box (`draw_button_box`) replaces
`draw_frame_control` at every call site whose listing calls the box slot;
`draw_frame_control` remains for the one screen that calls the frame slot
directly.

The form draw's string release is compiler-generated cleanup of a local
string. Rust ownership frees it, so it has no item.

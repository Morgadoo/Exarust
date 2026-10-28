# Design

The marker and the alert pulse every frame, so they are not pushed to the
window-sized GUI canvas (which would re-rasterise the whole window each
frame). Each indicator's layers (a rounded halo and the icon batch) are
built through the skin batch and `icon_batch`, moved to the corner of the
box holding their vertices and rasterised by `gui_draw::rasterise` into a
canvas of about 18 × 18 units (72 × 72 pixels at unit 4); a node shows
it between the compass and the bars. A canvas is redrawn only when its
layers differ from the last ones shown, i.e. when the pulse changes a
byte; a hidden indicator costs nothing.

The icon is cut from the stored (bottom-up) rows and resampled with
`exa_render::mipmap::resample_rgba8_to_size` and the atlas's pixel format,
then added to an 8-unit `IconAtlas` whose image is the layers' texture.
The atlas is rebuilt when the GUI unit changes.

The game keeps no skill record for the player in the running game, so
`StatusView::ready` stays false there; the rule is tested in `exa_ui`.

The form rules (close cascade, window steps, table copy) are pure: they
return steps or tables for a caller; the live screens close through
`GuiTask` and do not drive them yet. The cascade guards against a
malformed tree linking into itself with a depth limit of 64.

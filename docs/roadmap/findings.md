# Findings from the audits

Defects and stale claims that audit and review lanes report while they read
code. They are not fixed during Phase A (audits are read-only). Each becomes
Phase B work; its ledger row is `partial` or `missing`. No retail addresses
here; the rows in `context/re/ledger/` carry them.

| id | package | area | finding | Phase B |
|---|---|---|---|---|
| F1 | world-01 | terrain | The original neither draws nor casts terrain shadows from blocks whose composite state is blank (no composite: beyond about 44 m). Exarust draws and casts them. The shadow-draw doc names an "external-draw bit"; the assembly tests the composite state. | B4 world |
| F2 | world-01 | terrain | The composite set draw stays behind `EXA_TERRAIN_SETS=1`; the default draws every block layer by layer (DEC-6). | B4 world |
| F3 | world-01 | terrain | `StaticTerrain::shut_down` models the activation as the deactivation (inverted); test-only code. `CompositeSelector::remove` resets the block state, which the original does not. | B4 world |
| F4 | world-01 | lights | `shadow_links::track_lights` floods every light; the original floods only one light kind and puts the others in every cell (the port `fill_directional` is test-only). | B4 world |
| F5 | world-01 | scene | The live Scene registers no light aliases (they live in `RoomLights`/`shadow_links`); the doorway scissor is a view-list cull, not a pixel scissor. | B4 world |
| F6 | render-01 | GI | Voxel lights and shapes are injected in one pass and encoded once; the original blends each as its own pass. Spot lights get no matrix shadow; terrain levels lack the terrain block layout branch. | B4 rendering |
| F7 | render-02 | physics queries | Live sphere and swept-sphere queries use parry's closest point instead of the ported face tests (which are test-only); the push-out clamp against the plane is missing. | B4 physics |
| F8 | render-02 | meshes | The GPU mesh build has no morph-delta stream, no constant-colour fill and no modifier patch callback. | B4 rendering |
| F9 | powers-01 | power tree | Reopening the tree keeps the last domain for any body; the original only does so for the same player body, otherwise it opens on Mind. The bar toggle does not end Control Mind as the original's does. | B4 powers |
| F10 | powers-01 | sounds | The cast-fail sound (0.05 volume, 0.8 pitch) is not played by the throw and kinesis casts, its real callers. | B4 powers |
| F11 | physics-02 | bodies | Parking does not recurse down the node tree (held items are not parked); placement does not recurse into children; burn doses are never added live. | B4 physics |
| F13 | powers-01 | power tree | The glow link rings scroll the wrong way: one shader arm serves both the link and the glow-link programs, but the glow program subtracts the phase offset where the link adds it (a one-sign fix). | B4 powers |
| F12 | ui-01 | GUI skin | The close-mark and arrow icons are computed but never painted; scroll troughs and pressed/hover marks are never driven. | B4 UI |
| F14 | render-03 | render | exa_items::item_renderer::render_plan reads the item preview's subdivide bit (variant & 0x80000000) into Plan.subdivide, but nothing acts on it. The original smooths the preview scene (from the item GUI). | B4 |
| F15 | render-03 | render | The msm debug command in exarust main.rs register_debug_commands only logs a hint and does not smooth the live scene. | B4 |
| F16 | render-03 | render | Character-creation and arena hair is not smoothed: exa_character hair.rs marks it 'Not done'. The original's caller is SG_CharacterGen when flag is set. | B4 |
| F17 | render-03 | render | ModelLoader::smoothed_model ignores the scene walk's mesh flag 0x40000, masters-only rule and mode word filter. | B4 |
| F18 | render-03 | render | Character bodies get no GI surface points: gi_points_for returns nothing for skinned meshes, but the original voxelises them in mode 2 through. | B4 |
| F19 | render-03 | render | The live WESL hard-codes 8 shadow taps, 2 per super-sampled sub-frame. That is correct at the shipped settings only; other shadow-quality and supersample settings do not follow SdwSmps. | B4 |
| F20 | render-03 | render | There is no refraction in exa.wesl (no refraction strength, no BkgRfr01/02 background), and the translucent class-7 program draws with the default program. | B4 |
| F21 | render-03 | render | The texture program, program cache and uniform-location models (texture_program, shader_cache, shader_bindings::program_bindings) are present but have no live callers. | B4 |
| F22 | render-04 | render | Original quirk, not an Exarust bug: in the convolution set-up, a word outside the six layouts returns al = the low byte of the subtracted word. Words whose low byte is nonzero therefore carry on with an unfilled record. Its 0x80000 float branch is dead because that bit sits inside the 0x800cffff mask. Exarust refuses every unmatched word and records no decision for that. | B4 |
| F23 | render-04 | render | Possible defect: exa_game::thaum::kinesis_live mesh_distance asks nav_pick::kinesis_hit with arbitrary steering directions. kinesis_hit asks the mesh from the crossing (centre.y+radius-origin.y)/dir.y, so a near-horizontal steering ray gives an infinite crossing and can never hit a prop mesh. This is not confirmed against how the original TPwrForceKinesis_S calls it. | B4 |
| F24 | ui-02 | ui | hit_line: BaseForm::frame_code always passes wide=false. Retail reads byte (initial value 1, written by hwconfig and steaminput), which widens the title hit band from 4 to 6 units. | B4 |
| F25 | ui-02 | ui | Button caption hover arm: retail calls glow_text when state&0xf0000==0x10000; gui_paint::paint_line_button draws the plain off ink, marked as a guess. | B4 |
| F26 | ui-02 | ui | settings_form::paint_control ignores SliderDraw.scroll, so a marker-mode trackbar on a line-skinned form would draw nothing. | B4 |
| F27 | ui-02 | ui | Save screen combo box: retail draws the line skin's slot-33 rail (capped span plus thumb); save_screen::paint_combo uses draw_style_frames plus a filled arrow instead. | B4 |
| F28 | ui-02 | ui | The creator's picker markers use creator_screen::marker_layer, marked [guess], instead of the ported four-unit square (skin::draw_span_square, slot 26). | B4 |
| F29 | ui-02 | ui | draw_line_rail model: the hovered thumb alpha is forced to 255, where retail binds the GUI alpha byte (fade*255). draw_span_bar model: the cap inset is taken as unit, where retail reads skin field. | B4 |
| F30 | ui-02 | ui | BaseForm::rail passes owner_left/top 0 to refresh_line_scroll, where retail reads the owner's/ fields. | B4 |
| F31 | ui-03 | ui | exa_game::mind::runtime::raise_corpse logs 'no loot panel is open... (Exarust opens none on a corpse)', but loot windows on dead bodies now open live. The raise does not close the body's window itself; only close_changed_loot_windows takes it later, and the root focus is not done. | B4 |
| F32 | ui-03 | ui | Retail (Control Mind closing the loot window on one character) has no port. | B4 |
| F33 | ui-03 | ui | The journal's New and Erase buttons are drawn but not wired, and its text box is read-only, so Journal::add_entry, delete_entry and list_changed and the Tab callback never run. | B4 |
| F34 | ui-03 | ui | The heartbeat stops abruptly at 7 s and on resets. The original's 1 s sound fade-out, from menus, level changes and death screens) is missing, and reset_death_fade does not start it. | B4 |
| F35 | ui-03 | ui | hud::draw passes power_bar: false although Exarust has powers. Retail shows the bars while a power is being cast (flag. | B4 |
| F36 | ui-03 | ui | HudShowUntil is not cleared by any HUD reset zeroes the hold clock). Its 1000 ms callers (red health restored, quick-bar power) are not wired. | B4 |
| F37 | ui-03 | ui | The inventory and loot item panels are not saved on a game save. docs/UI.md calls this an Exarust choice, but there is no DEC entry for it. | B4 |
| F38 | ui-03 | ui | OpenContainers::is_open has no live caller. Retail's AI search (TAISearchActivity checks whether the player has the container open. | B4 |

Stale documentation found (fix with the next change to each file):
the terrain module's activation names; `tree.rs` module doc ("nothing offers
a power yet"); the GI texture cache and `follow_camera` comments; the
RFE_MeshTools "not wired" rating; several pack ratings that predate round 146.

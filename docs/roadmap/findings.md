# Findings from the audits

Defects and stale claims that audit and review lanes report while they read
code. They are not fixed during Phase A (audits are read-only). Each becomes
Phase B work; its ledger row is `partial` or `missing`. No retail addresses
here; the rows in `context/re/ledger/` carry them.

| id | package | area | finding | Phase B |
|---|---|---|---|---|
| F1 | world-01 | terrain | (Confirmed by review against the assembly.) The original neither draws nor casts terrain shadows from blocks whose composite state is blank (no composite: beyond about 44 m). Exarust draws and casts them. The shadow-draw doc names an "external-draw bit"; the assembly tests the composite state. | B4 world |
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

| F39 | render-05 | render | exa_physics::contact::swept_hit differs from RF_Mesh. The closest point comes from parry's project_local_point. The front-end reject uses r*r where the original uses (r-1.19e-7)^2 with a 1.19e-7 plane epsilon. The end-passed-beside branch is rewritten (its own comment says the original scales the start's offset). The original's return of the last hit face is not modelled. | B4 |
| F40 | render-05 | render | exa_character::blood::paint draws one random number per render vertex, where the original draws one per position element. Split vertices of one element can therefore get different blood values. It also uses a xorshift RNG instead of the engine's Random. | B4 |
| F41 | render-05 | render | Moving the character creator's Height/Age tracks does not re-morph the mesh live: morph_controller's evaluate_morph_positions and apply_morph_vertices have no non-test caller. | B4 |
| F42 | render-05 | render | EditMesh::duplicate and merge::assemble_editable have no live caller, so the chargen de-instancing copy is not wired. | B4 |
| F43 | render-05 | render | Tile mask pass: still unverified whether reading back the sRGB target returns encoded bytes (retail check pending). | B4 |
| F44 | render-06 | render | exa_assets::texture::TextureUnitCache (test-only model of says the bind resets GL_TEXTURE_LOD_BIAS to zero (field lod_bias_reset). The asm loads the bias global (movss) and passes that value. | B4 |
| F45 | render-06 | render | exa_game::parchment::texel_alpha reads the exact texel's alpha. The original decodes the alpha block of the 4x4 block that holds the point and returns byte 5, texel (1,1) of that block, so its hit test works per block, not per texel. | B4 |
| F46 | render-06 | render | The per-draw LOD bias is not followed. The original's TRenderer -> uses the current supersampled draw's level, which is 0 for item, icon and character-view draws. Exarust applies the settings-level bias to every material. | B4 |
| F47 | render-06 | render | ModelLoader::release_texture / TextureCache::release_record has no caller outside tests. Texture records are never released, so the original's frees at reference 0 (TMaterial destructor and others) do not happen. | B4 |
| F48 | render-06 | render | A texture name no pack holds gives no image in Exarust (texture_with_flags, Game::load_texture). The original falls back to the 1x1 grey placeholder,. | B4 |
| F49 | render-06 | render | The BC1-to-RGBA path gives alpha 0 for the 4th colour only for DXT1a. The original's RGBA converter gives transparent black for DXT1c as well. Its RGBA driver also skips levels under 4 texels, which Exarust decodes. | B4 |
| F50 | render-06 | render | The supersample target is Rgba16Float HDR and belongs to each camera. The original uses one shared GL_RGB8/GL_SRGB8 buffer sized to the largest view. | B4 |
| F51 | render-07 | render | Removing materials: the original swaps the list's last state into the hole and does not re-sort/. Exarust's order_blended_states re-sorts every blended state on AssetEvent::Removed, so after a removal blended draw order can differ until the next material add. | B4 |
| F52 | render-07 | render | The wall probe (exa_game player chase_camera obstructed closure) filters obstacles by the parts' last-frame ViewVisibility instead of the sector seen bit. A probe mesh that is frustum-culled inside a visible sector is skipped, where the original probes it. | B4 |
| F53 | render-07 | render | Cut-away faded pass: the original lays down depth for all faded opaque and alpha-tested entries before any colour. Exarust's per-part blended copy writes depth and colour in one draw, so fading meshes that overlap each other can occlude differently. | B4 |
| F54 | render-07 | render | exa_world::gi_volume floor_points takes its bounds from the floor triangles' min/max, not the +Floor node's stored bounds. This is probably equivalent for merged floors but was not proven. | B4 |
| F55 | render-07 | render | (displacement draw) sets a per-sub-frame uniform (base + i*step). Exarust draws the displacement once on one supersampled image, so the per-sub-frame value is not reproduced. | B4 |
| F56 | render-08 | render | Unranged level lights get PointLight range 500 m with (1-d/500)^4 falloff (light_bundle, UNRANGED). The original has no falloff for them: in, the value is 0 unless node flag 0x40000 is set and R > 0 | B4 |
| F57 | render-08 | render | LightSource::update, derive and DerivedLight::of have no live caller. So node flags 0x10000 (no GI) and 0x20000 (no second colour), and any runtime reach change, never reach the GI or the shadow links | B4 |
| F58 | render-08 | render | The lever and flicker steps scale the parent mesh's emissive colour, which the module doc calls 'Exarust's reading' (no DEC entry). They also skip the original's parent-kind-9 test and the alias copy loop | B4 |
| F59 | render-08 | render | TextureName::new truncates names over 16 bytes. The original keys each long name uniquely (13 bytes + 0x08 + a two-digit counter,, so two long names that share their first 16 bytes would share one texture record | B4 |
| F60 | render-08 | render | Textures are never released: ModelLoader::release_texture has no live caller, so manager records and strong handles stay alive for the whole session | B4 |
| F61 | render-08 | render | Cube-shadow casters with node flag 0x800 are not front-culled. The original switches to GL_FRONT at through | B4 |
| F62 | render-08 | render | self_shadowless ignores the light opt-out flag 0x200000 and the mesh keep flag 0x1000, and its walk does not stop at an object root | B4 |
| F63 | render-08 | render | The cube pool ignores ShadowDetail and the light's quality offset: faces are always 512 and the receiver bias is fixed at 0.995. The sun map is always 4096 with Depth32 instead of 16-bit depth at the ShadowDetail size | B4 |
| F64 | render-08 | render | Cameras after the first share the primary camera's sun softness filter; the original mirrors the softness per view | B4 |

| F65 | render-09 | render | exa_assets::model::library_material searches the project's material libraries first to last, then humanbase.rml. The original's providers answer last to first (a plug-in wins). A material name present in two libraries could resolve to a different material (rows and. | B4 |
| F66 | render-09 | render | The live shadow-depth path applies only the level-only bias rule. The 0x80000 'add' branch (both scales 1+level*0.005) is missing from the surface shader. shadow_bias_scales and geometry_node::shadow_scales are only called from tests. | B4 |
| F67 | render-09 | render | Light-to-mesh linking (shadow_links::link_weight) approximates the original's gated alias reach. The exact port, light_alias::reach and reach_plain, has no live caller, so lights without flag 0x40000 still get a distance falloff. | B4 |
| F68 | render-09 | render | LightAlias.shadow_faces and Scene alias sync values are computed in scene_nodes but nothing outside scene_nodes reads them. The TMotileAI call site of has no port. | B4 |
| F69 | render-09 | render | IconCamera::from_plan clamps the scene-fit distance to at least 1e-4; the original has no such floor. | B4 |
| F70 | render-09 | render | (scene-node framing for the creator's character icon through FLX_IconRender) has no port. This is DEC-3 domain. | B4 |
| F71 | render-10 | render | Post bloom targets: the asm ORs the caller flag with the sRGB-target switch, which is 1 in retail, so all three bloom targets are SRGB8. Exarust's bright and blur_x targets are Rgba8Unorm (exa_render/src/post.rs BRIGHT_FORMAT), so the 8-bit quantisation of the first two passes differs | B4 |
| F72 | render-10 | render | Spline fade: the original computes the alpha byte once from the first vertex's index and writes it to both vertices, and divides by the segment count given at begin. exa_ui::tree_draw::link_ribbon passes row*2+1 for the second vertex, divides by points-1, and does not zero the join term for modes 1 and 2 (the test-only splines::SplineStrip has the right rule) | B4 |
| F73 | render-10 | render | The bloom blur in post.wesl uses a fixed 16-weight table. bloom_weights/bloom_kernel (linear-path scale level*0.016+1) are test-only, so the weights do not follow the constructor's tap count | B4 |
| F74 | render-10 | render | Screen fade and desaturation fade, have no live caller (FadeControl is test-only), so the game-main fades and the bubble-effect desaturation never animate | B4 |
| F75 | render-10 | render | (clears the bloom target to a flat colour before the item-view capture) has no counterpart | B4 |
| F76 | ui-04 | ui | Skill screen: in the original, any key on the revealed ring closes it (slot 37,. In Exarust no key reaches the screen, so only the right button closes it. | B4 |
| F77 | ui-04 | ui | Skill screen: the open routine's toggle (an open screen closes) is skipped by open_skills, and SkillScreen::begin_hide is never reached on the live ring (page 0). | B4 |
| F78 | ui-04 | ui | SkillScreen.experience is never set non-zero live, so spend_skill_experience never runs. | B4 |
| F79 | ui-04 | ui | BaseForm::wheel drops the horizontal answer and skips the pointer refresh that the original calls after a wheel scroll. | B4 |
| F80 | ui-04 | ui | BaseForm::release ignores the maximise/restore title marks and the horizontal snap from release_actions. | B4 |
| F81 | ui-04 | ui | FormFade::start_hide does not clear the global form slot, does not call (state & 0x20) and does not request a redraw. FormFade::start_show does not make the form visible or raise it. | B4 |
| F82 | ui-04 | ui | App deactivation: there is no GUI focus handoff to the root task when the window loses focus. | B4 |
| F83 | ui-04 | ui | Resize-edge frame codes and grip drags are not wired in live forms (the line skin gives no resize codes). | B4 |
| F84 | ui-05 | ui | Live combo lists (option_forms::HubForms::open_list, save_screen::list_px) use controls::drop_down_place, which ports the open/save dialog's routine (cuts to the rows below or opens above using a row count). Retail combos size the list with TDropDownList slot and place it with: below the owner, or above by the list's whole height when it would leave the screen, with x clamped to screen_w - w - 2 a | B4 |
| F85 | ui-05 | ui | gui_paint::paint_combo_front never reads ComboDraw.editing: in the state 0xf0000 == 0x10000 arm the original draws the combo text through the skin's glow text, not through the edit run draw | B4 |
| F86 | ui-05 | ui | paint_combo_front shows the selection on state 0x400 alone; the original also needs the owner form's word bit 0x10 | B4 |
| F87 | ui-05 | ui | option_forms::paint_extended TEXT_FIELD always paints the class-5 label frames with the style forced to 0xf0. The original draws the skin control back (slot and draws the inner frame (slot only when the field's own style has a 0xf0 bit | B4 |
| F88 | ui-05 | ui | Colour swatch back draw: option_forms paints a panel plus a solid inner colour, not the skin's control fill (slot, skin::draw_control_fill has no caller) | B4 |
| F89 | ui-05 | ui | Editable combos (the journal's New Entry, the merchant boxes) never get the caret press or drag: option_forms::focused_field accepts only TEXT_FIELD, so the combo's state-0x20000 press and its 0x30 drag have no Exarust path | B4 |
| F90 | ui-05 | ui | Neither the text field nor the combo box nor the swatch handles the 0xc0/0x100 gamepad accept (Steam on-screen keyboard, open list, owner call with 0x800) | B4 |

| F91 | ui-06 | ui | exa_formats::hw_config::Screen::settings_lines writes 'InputMode = 51' for the controller; retail writes 'InputMode = 3'. Behaviour is the same because the reader treats any positive value as the controller, but the file text differs. | B4 |
| F92 | ui-06 | ui | HardwareConfig::apply sets live.shadow_detail but nothing live reads it: Apply's shadow-detail push to the renderer (RF_ShadowMap is missing. | B4 |
| F93 | ui-06 | ui | First-run auto-detection (SG_GameMain ->(1): benchmark or Deck preset, then Apply writing the file) is not done. flow::BootTask names it but does not perform it; --benchmark writes nothing. | B4 |
| F94 | ui-06 | ui | SuperSampling is a bool, so super-sampling levels 1, 3 and 4 render the same; only the texture LOD bias follows the level. | B4 |
| F95 | ui-06 | ui | The quality>5 shading flag (exa_render::quality::shading_high) has no live reader. | B4 |
| F96 | ui-06 | ui | Bindings::key_name returns 'Left mouse'/'Right mouse' for codes 1/2; retail's key-name routine gives an empty string for 0x8001/0x8002. | B4 |
| F97 | ui-07 | ui | GuiTask::set_cursor stores GuiTask.cursor, but nothing reads it. system_cursor::CursorTable::show has no live caller, so the window pointer never changes shape over grips or text | B4 |
| F98 | ui-07 | ui | PointerState.limit stays 2: gui_core::drag_limit has no live caller. The original sets max(2, round(h/540)) on every level switch, which is 4 at 2160 px, so the drag threshold is too small on tall windows | B4 |
| F99 | ui-07 | ui | The glyph sheet GlphCommon_rfi is never loaded into the large icon atlas. Only the constants gui_rescale::GLYPH_SHEET/SHEET_ICONS exist | B4 |
| F100 | ui-07 | ui | The pointer-confine wrapper (ClipCursor) has no Exarust counterpart (no CursorGrabMode anywhere). Its callers include TPlayerBehaviour and RF_Navigation | B4 |
| F101 | ui-07 | ui | Message boxes with the 0x80 flag (emblem COMPANY_NAME_BOX_WORD 0xc1) are never made the GuiTask modal form. HubForms keeps them on top instead | B4 |
| F102 | ui-07 | ui | GuiTask.gamepad and DropState.kind are never set outside tests, so the gamepad focus arms/ac0) and the whole drop path, are dead code live | B4 |
| F103 | ui-07 | ui | Every live TextBox measures with no pictures, so a [rimg] tag in help or journal text adds no height | B4 |
| F104 | ui-08 | ui | The glyph builder never reads GUISymbl. exa_ui::gui_level passes None to build_glyphs, so glyph 1 is the fallback cross where retail shows the pack's GUISymbl 48x48 symbol. | B4 |
| F105 | ui-08 | ui | In layout_records, only the trackbar's class init runs. The original calls every class record's init hook during layout. | B4 |
| F106 | ui-08 | ui | The push-button hint arm leaves out the tooltip call that the original makes when the GUI flag is set. | B4 |
| F107 | ui-08 | ui | There is no live port of, which finds the owner's view, runs and clears the job. It is called by the attach finalisers and the equipment-effect routines; only the FinishSteps.callback flag exists, and it is used in tests only. | B4 |
| F108 | ui-08 | ui | The live table row hits (match_board TableLayout::row_at, hub_forms::manage_table_row) ignore the scroll's first visible row and the growing-table line that the original row hit uses. | B4 |
| F109 | ui-08 | ui | The character view's background wash, race-share easing and frame counter are ported but never called. | B4 |
| F110 | ui-08 | ui | gui_controls::set_steps computes value = at/steps in f32. The original divides in double and then narrows, so the rounding can differ in rare cases. | B4 |
| F111 | ui-09 | ui | : leaving the title for the creator or class select clears the viewport's version-line flag in the original. Exarust's creator_screen::open_screens does not clear ViewOverlays.title (only Step::SessionTask(true) does), so 'V 0.9.5.2g' likely stays drawn over the setup forms | B4 |
| F112 | ui-09 | ui | : NEW CAREER opens the emblem generator in the original. Exarust maps MenuCommand::NewCareer to the free creator in arena mode (CreatorEntry::arena), so the emblem form never opens from the menu | B4 |
| F113 | ui-09 | ui | : in the original, dragging off the pressed menu row sets the selection to 0. menu_screen::input keeps the old selection | B4 |
| F114 | ui-09 | ui | : Up/Down go through the select slot, which plays MenuSel (200 ms limit). MenuScreen::move_selection plays no cue | B4 |
| F115 | ui-09 | ui | : back from page 1/8 plays MenuClick at pitch 1.0 in the original. MenuScreen::back plays nothing | B4 |
| F116 | ui-09 | ui | : the class-select show warps the pointer to (W/2, round(f*370)+(H-f*1024)/2) and selects the first card with a pad. open_screens does neither, and class_select_place is test-only | B4 |
| F117 | ui-09 | ui | /: [rimg=NAME] pictures in text boxes are never drawn. Every live TextBox::draw call passes a picture lookup that returns None, so pictures get size 0, no texture is loaded and the text is not pushed down | B4 |
| F118 | ui-09 | ui | : the view-to-pixel conversion snaps a point at or behind the eye to the screen edges and returns depth -z. player::window_point (Bevy world_to_viewport) returns no point instead | B4 |
| F119 | ui-09 | ui | TCompactSkillUI (11 functions, 5838 B): the ported compact_skill module is not mounted. creator_screen::paint_equipment draws the skills panel as an empty panel | B4 |
| F120 | ui-09 | ui | : the read-only text field navigation (navigate_key) is never driven because every live text field is editable (style bits 0xd) | B4 |

| F121 | ui-10 | ui | destination.rs: JourneySave (the adapter journey's save) is written but nothing outside tests reads it; TravelStep::stop_controls is computed and ignored (retail stops the player's controls via /. | B4 |
| F122 | ui-10 | ui | Destination screen leave: retail slot 37 closes on any key (it does not read the key code); Exarust's portal_pointer passes only Escape. | B4 |
| F123 | ui-10 | ui | DestinationScreen::close does not pop the controller action layer when form word has bit 0x20). | B4 |
| F124 | ui-10 | ui | open_screen passes the markers as an empty Vec, so every row's marker word is 0. The destination mark goes through journey.device unconditionally, while retail requires marker bit 31 and an existing record. | B4 |
| F125 | ui-10 | ui | Manual topic-list pick: also gives the text box focus, -1). Exarust's list click calls Manual::select directly, and list_picked is test-only. | B4 |
| F126 | ui-10 | ui | The manual's close is not driven from the menu's hide routines (retail TMainMenu / call. | B4 |
| F127 | ui-10 | ui | Text renderer: no live caller supplies a picture lookup, so [rimg] pictures in the manual/journal/item cards are never drawn. | B4 |
| F128 | ui-10 | ui | SG_GUI_Skills (compact skill form for character creation) has no live port; the skill window page arm (SkillScreen::open with page > 0) is never driven. | B4 |
| F129 | ui-10 | ui | XA_HelpSystem show_tip (one-time tips) has no live caller: the inventory mode-8, transition-screen and arena tip calls are not wired. | B4 |
| F130 | ui-10 | ui | ColourSwatch::command (pad/Steam actions on the palette popup) has no live caller. | B4 |
| F131 | ui-10 | ui | The emblem form, palette popup and colour picker are reachable only through debug openers (--open-hub-window company / picker); there is no New Career front end. | B4 |
| F132 | ui-11 | ui | TDropDownList: the live combo popup (option_forms popup_input) handles the pointer only. DropDownList::command (keys and pad: 0x100 choose, 0x200 close, step up/down) has no caller, and a list cut at 16 rows gets no scrollbar hit or drag (hit and drag slots and are bypassed through row_at) | B4 |
| F133 | ui-11 | ui | Transition screen: retail's hit slot returns -1, so the form takes every pointer hit while it is up. The live Bevy overlay node captures nothing (transition_hit is test-only) | B4 |
| F134 | ui-11 | ui | Transition 'TO BE CONTINUED' hold: begin_continued, continued_text_origin and menu::draw_transition_fade have no live caller | B4 |
| F135 | ui-11 | ui | The live KeyDef hotkey dispatch (commands::queue_binding_lines) fires every binding whose modifiers are held. Retail's TGUITask key slot runs only the best modifier score (earliest wins a tie) and forwards unmatched keys to the parent layer | B4 |
| F136 | ui-11 | ui | The warm-up screen's per-frame step only counts the 29 rows (warm_row). Retail steps the GPU program warm-up there. No DEC entry covers this | B4 |
| F137 | ui-11 | ui | The guiskin atlas is loaded once at Startup at its file size. Retail resamples it to 40x20 units and reloads it on every GUI scale change | B4 |
| F138 | ui-11 | ui | Modal screens (key capture, warm-up): settings_form marks the full-window black backdrop and the palette text shade as guesses | B4 |
| F139 | ui-12 | ui | The Company Status window draws no company emblem: status_emblem_place has no non-test caller, while the original draws it at. | B4 |
| F140 | ui-12 | ui | In the four confirmation dialogs (reset to portal, delete, dismiss, yes/no), a press off the controls does not close the dialog with 0x40 (slot 27). dialog_click is test-only. | B4 |
| F141 | ui-12 | ui | The arena help popup (THelpScreen via XA_HelpSystem, opened from XA_Arena, TTransitionScreen and SG_GameGUI) is not ported. The help-seen mask is never written to settings (TipStore has no write). | B4 |
| F142 | ui-12 | ui | When the manual is destroyed while the arena is up, the arena form's mode is not set to -1 (help_destroy_mode is test-only). | B4 |
| F143 | ui-12 | ui | No live GUI record carries tooltip hint text, so the default tooltip never shows in play (only --debug-hint sets one). | B4 |
| F144 | ui-12 | ui | Controller actions (Steam Input slot 30) are not sent to forms: no message box confirm or cancel by action, and no manual page scroll or close by action. | B4 |
| F145 | powers-02 | powers | The Sense Minds casts (mind_sense, mind_sense_persistent) return MindRequest::SenseOverlay, but nothing consumes it. Casting Sense Minds shows no sight overlay and no spheres; only the power entry and its ticking are live. | B4 |
| F146 | powers-02 | powers | The particle body-contact pass (Emitter::contact and take_dose, class slot 31) is never driven from the physics step. Particles pass through bodies and the dose never reaches limbs. | B4 |
| F147 | powers-02 | powers | The fire emissive only reaches the host's first-part material (fire::drive_hosts). The original also writes the parent node's scale and every geometry child's. particles::apply_emissive does this but has no caller. | B4 |
| F148 | powers-02 | powers | (end Spirit Form or Control Mind) is not called from the bar toggle or from the character AI. Only the blow break and the cast end_control are wired. | B4 |
| F149 | powers-02 | powers | Calm leaves out the original's write one past the end of the contact array. The code comment says it is not reproduced, but there is no DEC- entry for it. | B4 |
| F150 | powers-02 | powers | Mind restore constructors and restore_mind_power are test-only. Persistent Mind powers saved with a character are not rebuilt on load, and their focus reserve is not re-taken. | B4 |
| F151 | powers-02 | powers | Particle quad mode 0 (screen_quad), the UV_8 variants and flat colour are ported but no live template sets their flags, so those branches never run. | B4 |

Stale documentation found (fix with the next change to each file):
the terrain module's activation names; `tree.rs` module doc ("nothing offers
a power yet"); the GI texture cache and `follow_camera` comments; the
RFE_MeshTools "not wired" rating; several pack ratings that predate round 146.

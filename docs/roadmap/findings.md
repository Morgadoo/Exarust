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

| F152 | powers-03 | powers | Torrent sim-object slot 32 is not ported: tick_torrents has no cloth walk | B4 |
| F153 | powers-03 | powers | Torrent voices are never played: Torrent.sounds (charge loop, stream loop volume/pitch bumped by walk_torrent_mesh) is written but nothing reads it | B4 |
| F154 | powers-03 | powers | draw_blasts skips charging blasts, but the original builds the ribbon strip in every state, with a charge colour | B4 |
| F155 | powers-03 | powers | Blast and torrent part contacts do not write body byte (blast only) or the kind-0x10 part's node 0x80 bit / part word; the knock is a flattened begin_knockback stand-in | B4 |
| F156 | powers-03 | powers | Torrent part contact wakes the part in the original; Exarust touches the whole body | B4 |
| F157 | powers-03 | powers | Level static contacts (blast, torrent walk triangles in rapier's order with no kd-tree, so the push-out order can differ from the original | B4 |
| F158 | powers-04 | powers | Throw cast: in the original a failed cursor pick plays ThaumFail01 (fail sound. Exarust's ForceStart::ThrowMissed creates nothing and plays nothing (thaum/cast.rs create_force). | B4 |
| F159 | powers-04 | powers | Kinesis-to-throw handover: the original also needs the held body's holder count (sim below 1. Exarust's kinesis_holds only checks for a non-ending kinesis with a target; force::kinesis_hands_over has no live caller. | B4 |
| F160 | powers-04 | powers | Decorative shell is missing. The Force awakening game events (0xC10EC1/0xC10EC3) put a scale-3 _ForceAura01 shell on the player in the original; game_event_live only logs 'its body effect is not ported'. | B4 |
| F161 | powers-04 | powers | Level electricity devices (exa_world::mechanisms::start_device_arc -> exa_world::arc::spawn_arc) use exa_render::electricity's lay-out-once jitter model instead of the TTendrilEmitter simulation. Level arcs therefore get no tick, branching, level clip, body strike, follow axis (slot 27) or light flicker from ArcEmitter. The electricity.rs header says this 'reproduces the presets... exactly', which | B4 |
| F162 | powers-04 | powers | Arc frame: draw_arcs calls ArcEmitter::frame with fade 0.0, so the node fade never scales the voxel or GI light. | B4 |
| F163 | powers-04 | powers | Persistent Force powers saved with a character (shell, strikes) are never restored: power::restore_persistent, the only caller of force::restore_force_power and Shell::new/Strikes::new in that path, has no non-test caller. | B4 |
| F164 | powers-04 | powers | Many Force effect CPU models are test-only and the GPU materials stand in for them: effect_material::dress, GiPrograms, ShaderBook, effect_pass::PassKind, visuals::scroll_offsets, arcs::attach, add_to_world, remove_from_world, follow_parent, follow_axis. | B4 |
| F165 | powers-05 | powers | TFireEffectRow is also the item's release hook, called from the hand-change routine. Exarust has no extinguish-on-release, so a lit blade that leaves the hand is only handled by the once-a-second check or by its entity being despawned | B4 |
| F166 | powers-05 | powers | item_ability::ability_tick returns early when no held item has an ability, so HeldModifier (Flaming) and BurningGrip are not reset in that tick after the sword leaves both hands (not verified against the original's moveset recompute) | B4 |
| F167 | powers-05 | powers | Barricade is silent: cast::create_force passes hum=None and sectors=None to add_to_world, so the AI entity has no 6 m sector; the impact sound and the hum gain/pitch are computed but never played | B4 |
| F168 | powers-05 | powers | Barricade::destroy has no non-test caller; the teardown is done ad hoc in tick_barricades | B4 |
| F169 | powers-05 | powers | The vanish and Long Door leave no tendril after-image (TVanishEffect calls; VanishNode.image is only a bool | B4 |
| F170 | powers-05 | powers | spawn_body_wisps uses wisp size 12 and does not scale it by the body's size factor as does | B4 |
| F171 | powers-05 | powers | veil::restore_shells, clear_shells and prune_meshes are test-only; the vanish, Long Door and summon activations never apply the shell restore/clear walks or the mesh prune order | B4 |
| F172 | powers-05 | powers | FlameRowMaterial is shared by all rows, and its Anim follows the first row's fade only | B4 |
| F173 | powers-05 | powers | The flame row blend is a guessed additive mode (flame_row.rs doc); there is one draw instead of one per supersample scissor branch | B4 |
| F174 | powers-06 | powers | exa_world spawn_model spawns a fire for every model fire marker without checking the scene's world-fires flag 0x10000 that the original marker importer checks (fire::import_marker is test-only) | B4 |
| F175 | powers-06 | powers | Level device arcs (exa_world::arc) bind a white 1x1 stand-in texture instead of _ElecMat01's emissive map; the power and weapon arcs in thaum::live load the real texture | B4 |
| F176 | powers-06 | powers | The lightning mace hit runs charge and spark burst immediately instead of deferring them to the event queue, marked '[guess: Exarust runs them at once]' with no DEC entry | B4 |
| F177 | powers-06 | powers | Barrier::tick_place (slot 28 sub-frame node placement) has no live caller | B4 |
| F178 | powers-06 | powers | A closed constant displacement shell is removed rather than kept (open r162 gap, render step | B4 |
| F179 | powers-06 | powers | ShellEffect / effects::shell::tick_shells is registered but nothing spawns a ShellEffect component; the live displacement shell runs through vanish_live instead (dead system) | B4 |

| F180 | powers-07 | powers | exa_ai::combat::barricade_place (the live barricade avoidance) differs from the original slot 15. Reach is body.reach_cm (x1.4 above scale 1.3) where the original uses arm*1.5 or 1.4 plus base. The keep distance is reach+140 instead of the AI record's keep word. The window is never doubled for the alert state. effects::danger::avoid_barricade has the right rule but only test callers. | B4 |
| F181 | powers-07 | powers | Dome::frame Phase::Ending resets the hum to 0.2/0.75 before adding the glow. The original leaves the words as they are in state 2 and adds the glow on every frame, so they accumulate. | B4 |
| F182 | powers-07 | powers | Shock's thmnrg_chrga and thmnrg_shock sounds never play: ShockStep::charge_sound and release_sound are read by nothing live. | B4 |
| F183 | powers-07 | powers | The Long Door's start and move sounds never play: LongDoor::start_sound and move_sound have only test callers. | B4 |
| F184 | powers-07 | powers | The teleport bubble's 5.6 s post-process flash (BubbleStep::post_process) is computed but read by nothing live. | B4 |
| F185 | powers-07 | powers | Barricade sight copies only seen.fear into the contact's alarm. The near, reach, urge, face and raised-alarm words are dropped. | B4 |
| F186 | powers-07 | powers | ripple_strikes picks the aura on the striker's primary-slot weapon. The original matches the hooked object's node, so a blow from another held item does not ripple that item's own aura. | B4 |
| F187 | powers-07 | powers | Running persistent powers are neither saved (save_power, slot 17) nor restored (restore_persistent, by the live save and load path. | B4 |
| F188 | powers-07 | powers | The wave's charge loop voice (force_wave::wave_loop) and ribbon mesh (mesh_plan) are unwired, so a force wave is silent while charging and is never drawn. | B4 |
| F189 | powers-08 | powers | mind::runtime::tick_minds calls mark.control(0, 0.0) for the Loyalty marker. The strength drift after 800 ticks uses relation 0 instead of the real relation (original | B4 |
| F190 | powers-08 | powers | Mind powers (Spirit Form, Control Mind, Loyalty, Command Dead) live in MindBody.powers and are never passed to PowerBar::mark_running/load_running, so their bar slots are never lit. SlotMark::Lit/FadeOnEnd exist but are unused for them | B4 |
| F191 | powers-08 | powers | Shock's ShockStep.charge_sound/release_sound are never played by thaum::cast::tick_casters | B4 |
| F192 | powers-08 | powers | The fire row's FireS02 loop: frame_rows walks FlamePitch, but no audio system plays the loop | B4 |
| F193 | powers-08 | powers | Displacement tendrils draw with DispTendrilMaterial (_TDispMat01 surface program), whereas TTendrilGeometry's own program is effectforce_v/effectsmoke_f. ForceRibbonMaterial::smoke has no live user | B4 |
| F194 | powers-08 | powers | exa_ai::combat::blast_place writes only `danger` (close stays 0), while the original writes the same value to two score words (+4 and +8). The PlaceScore word mapping needs checking | B4 |
| F195 | powers-08 | powers | see_force_effects builds Viewer { look: facing, facing } and copies only flags and fear into the contact. The look axis and the near/reach/urge/face words are lost | B4 |
| F196 | powers-08 | powers | Vanish's object is an audio emitter (freed with and used as the move sound's source), but displacement::VanishNode models it as a 'light' | B4 |
| F197 | powers-08 | powers | The after-image of Vanish/Long Door is only a bool flag that nothing draws | B4 |
| F198 | powers-08 | powers | possess does not copy the caster's three body vectors// or the node words.. to the target; it copies only the aim | B4 |
| F199 | powers-08 | powers | The spirit screen hook writes neither the scene-env colour/value nor the noise picture upload, so its stop has nothing to restore | B4 |
| F200 | powers-09 | powers | Force Barricade: ending the power by event (clicking the wall again via force.rs Barricade arm power.state=2, or status broadcast) drops the ActiveEntry in thaum::cast::tick_casters without calling Barricade::end / setting TOLD_TO_END; only tick_barricades' range check (power_ends(0,..)) ends walls, so a cancelled barricade keeps standing | B4 |
| F201 | powers-09 | powers | Mind powers (MindPowers list) never feed PowerBar::mark_running/load_running: persistent Sense Minds, Mask Mind, Command Dead caster, Loyalty, Control Mind slots are never lit/faded on the bar (SlotMark::Sense/Mask/FadeOnEnd have only test callers) | B4 |
| F202 | powers-09 | powers | Command Dead marker end: no counterpart for (swap-remove of the body from the global list | B4 |
| F203 | powers-09 | powers | Sense Minds persistent: MindRequest::SenseOverlay is never consumed and MindSense.overlay is unread, so no overlay shows or fades | B4 |
| F204 | powers-09 | powers | Power save/load (save_power, restore_persistent, load_power) has no non-test caller: running powers are not saved or restored | B4 |
| F205 | world-02 | world | cross-room light lists: Rust RoomLights::propagate feeds only the terrain block ranking (terrain::live::room_light_lists). The renderer's room light lists (shadow_links) never run it. The original runs it at the end of a save-game level load. Round 159 says REA shows it has no load-time effect, but this needs re-checking. | B4 |
| F206 | world-02 | world | : the original writes the character's cell bytes at/. Rooms::cell_bytes has no live caller, and the room_view node hook takes the room from the position, not from those bytes (round 159 gap). | B4 |
| F207 | world-02 | world | : the KdTree root box comes from the triangle vertices, not from the mesh's stored box (mesh/. The two differ if a stored box is not the vertex box. | B4 |
| F208 | world-02 | world | : cloth sweeps (TVCCloth, the only caller) visit triangles through a TriangleGrid in list order instead of tree leaf order. Pushes move the sweep end, so the order can change the result. | B4 |
| F209 | world-02 | world | : the room occluder depth-only pre-pass is replaced by opaque black draws. This adaptation has no DEC entry. | B4 |

| F210 | world-03 | world | exa_audio::mechanisms keeps mechanism voices by entity and was not seen releasing them on despawn. A door leaf despawned mid-creak may keep its voice; the original destructor releases it. Not confirmed. | B4 |
| F211 | world-03 | world | On Record levels in Auto sun mode, level_lighting_scene tests lights == 0 only. The original's rule also needs no grid rooms and removes the sun when there is more than one room. | B4 |
| F212 | world-03 | world | Plants::load falls back to tint [0.72,0.68,0.76] when a palette has no plants and uses three channels; the original copies four floats only when plants exist. | B4 |
| F213 | world-03 | world | scale_rule_heights (the original constructor scales height-rule bounds by 100) has no live caller. Whether RuleTree's height inputs are already in matching units was not checked. | B4 |
| F214 | world-04 | world | exa_world::mechanisms::order_doors drops the Sounds and object word returned by mechanism::open_doors / close_and_lock_doors / toggle_doors. A signalled open plays no unlock sound, a close-and-lock plays no lock sound, and the node's object-word bit 0 is never saved. Only open_doors_under sends LockSounded. | B4 |
| F215 | world-04 | world | Lever timed return: Lever::return_word has no caller, so the remaining return time is never written into the object word (bits 16+). A save made during a return loses it. | B4 |
| F216 | world-04 | world | The load-time switch-off and daylight put-out (put_out_light) only zero LightSource intensity on the node and its direct children. The fixture's part flag 0x10/value and class 0x41 children are not touched, and the light search is not depth-first (find/daylight_target are test-only). | B4 |
| F217 | world-04 | world | Operative records missing at load are not reported: the original logs 'Operative data not found: <name>'. | B4 |
| F218 | world-04 | world | The tileset load does not log 'Failed to load tileset: <name>' and silently uses the fallback set. | B4 |
| F219 | world-04 | world | AI door handling (exa_ai::path::pass_door/give_up_door -> door::toggle/open_span) has no live caller, so NPCs never toggle doors as TMotileAI does. | B4 |
| F220 | world-04 | world | The grid floor/wall transparent-vertex channel (mark_transparent_triangles), set-node activation (link_grid_nodes) and 20 cm piece subdivision (subdivides_piece_child) are ported but never called by the live grid spawn. | B4 |
| F221 | world-04 | world | No Rust port of the T3DNode instance sync from master (class slot 17: scale-ratio box/radius, geometry clone). | B4 |
| F222 | world-05 | world | exa_world::scene_nodes::Scene::light_reaches returns true for the scene's own sector, marked in code as a guess. The original scene sector has a frame block of its own. | B4 |
| F223 | world-05 | world | exa_game::teleporters::DeviceReads::decide measures the device reach from the user's root (feet), marked in code as a guess for the original's body point. | B4 |
| F224 | world-05 | world | Level teardown (level_lifecycle::teardown) has no counterpart to the original's reparent walk or its streamer drain. Nodes shared with holder scenes are not re-owned. | B4 |
| F225 | world-05 | world | exa_world::presets::PresetTable is never built by the running game, so the start-up linear-lighting 0.75 scale and the empty ambient fill of the six presets do not apply. | B4 |
| F226 | characters-01 | characters | Hit and foley material: exa_audio::hits::impact_play and exa_audio::footsteps call exa_combat::sounds::part_hit_sound and foley with golem and skeleton hard-coded to false, so golems and skeletons get the flesh or armour sets. | B4 |
| F227 | characters-01 | characters | exa_audio::loops::LoopSources reserve and free have no caller outside tests. The original creates and frees the voice and body sound sources in the AI tick, revive and death,. | B4 |
| F228 | characters-01 | characters | Nothing in exa_ai or exa_combat writes the grip mode. The original's AI grip-mode writers (SG_CharacterAI, TMotileAI and TFireEffectRow have no counterpart. GripLinks::set_mode is test-only; animate_grips has its own inline copy of the logic. | B4 |
| F229 | characters-01 | characters | ReplaceHeld is requested only by limp::place_on_ground and the Long Door. Arena spawns, save loads, level changes, AI and player teleports never re-place held items. | B4 |
| F230 | characters-01 | characters | Wounds: the immediate paint uses radius squared x 6 ((30r+10)^2*6). No x6 factor was found in Exarust, which maps this to BladeBlood and stain_weapons. | B4 |
| F231 | characters-01 | characters | is listed as a cleanup stub in the motilepart11 table, but it is a deferred apparel-mesh wound paint job (RF_Mesh, then FreeMem). | B4 |

| F232 | characters-02 | characters | exa_combat::burn::Burn::add_dose has no live caller. Nothing writes Burn.strength, so burn_upkeep runs but fire particles never burn anyone (retail: brazier and floor-fire damage over time). | B4 |
| F233 | characters-02 | characters | weapon_record::step_reach rounds 80*sqrt(scale) in f32 before the double-precision factors and applies the feral factor only when feral > 0. The original multiplies the sqrt-scale word last, in double, and applies the factor when feral >= 0 or NaN. | B4 |
| F234 | characters-02 | characters | Holder.half_sword and Holder.weightless are never set on the live path (CharacterStats::holder uses..AVERAGE), so the half-sword hold branch of the holding pass never runs. | B4 |
| F235 | characters-02 | characters | The assign_tree_material doc says 'a leaf part takes it', but the original gates the write on part word < 1, not on having no children. Not verified further. | B4 |
| F236 | characters-03 | characters | exa_assets::model::merge_dressed always passes occlude:false, so the generator's occlusion alpha (exa_assets::merge::occlusion_alpha) never runs in the game; only tests set occlude | B4 |
| F237 | characters-03 | characters | exa_character::factory::PIXEL_FORMAT is fixed at 2, so the skin job side is always 512; the original reads the texture-detail setting (1024 from 3 up) | B4 |
| F238 | characters-03 | characters | exa_character::offhand::insert_equipped skips Attach::Torch: an off-hand torch and its light effect are not attached by the equip/attach path | B4 |
| F239 | characters-03 | characters | hair::attach_hair tints every part of a style. The original gives the hair material only to parts with opacity > 0.5 and writes owner word per node (exa_assets::hair::hair_material_part is test-only) | B4 |
| F240 | characters-03 | characters | Living placed characters' saved poses (tree and flat) are not restored; pose::place runs only for dead placements (viewer: c.dead && !c.fall) | B4 |
| F241 | characters-03 | characters | rebuild::dress_plan fills GarmentMerge.merge (runtime::merge_order) but nothing live reads it | B4 |
| F242 | characters-03 | characters | Creator roles (spawn::creator_role) and limb scale (look::limb_scale) have no live caller, though the original's body build applies both | B4 |
| F243 | characters-03 | characters | Held-item mesh flag 0x10000 (runtime::Body::mark_held/unmark_held/mark_hands) and the lit held-item material raise (runtime::raise_material_flags) have no live caller | B4 |
| F244 | characters-05 | characters | skin::GroupStack (push limit, overflow error, push_empty past the limit) is only called from tests. The live SkinPass passes frames down by recursion with no 16-frame limit and no overflow error. | B4 |
| F245 | characters-05 | characters | Original and the other builder loops read triangle indices from the incidence table as u16, so a mesh with more than 65535 triangles would wrap. Rust uses full indices. This only matters on oversized meshes. | B4 |

| F246 | characters-06 | characters | Skill unlearn: the faithful port (CharacterData::unlearn_level plus unlearn_dependents) has no live caller. The live skill screen (exa_ui::skill::unlearn_skill_level/unlearn_in_block) runs a different cascade: it walks from the last funded level down, by level, while the original rescans the first four slots from 0. Results may differ from the original. | B4 |
| F247 | characters-06 | characters | The arena's arsenal Holder::remove_everywhere forgets an item in carried, one hand pair, worn and loadouts. The original also clears the second hand pair and the staged list. CharacterData::remove_everywhere does that but is test-only. | B4 |
| F248 | characters-06 | characters | Level-placed characters (viewer spawn_placed_characters) take no health pools from their record. The original import spawn builder writes red and yellow. | B4 |
| F249 | characters-06 | characters | Morph transfer barycentrics: the live path uses closest_on_triangle (region test, clamped). The original uses a plane projection with an /x///y/ sum branch. The faithful morph_barycentric is test-only. | B4 |
| F250 | characters-06 | characters | Morph diffusion in MorphBody::transfer skips zero-distance neighbours, where the original divides by zero. | B4 |
| F251 | characters-06 | characters | Many CharacterData rules have no live caller: mark/unmark equipped, stow/unstow hands, take_equipped, drop_surplus_refs, remove_item (trade), remap_items, retain/release, strip, can_spend_level, shared_item_refs, clear_items, push_carried/add_carried_unique/pick_up, equip_weight, worn_weight, remove_worn. The live inventory, trade and AI paths do these jobs with their own code or not at all. | B4 |
| F252 | characters-06 | characters | save_kit::instantiate_prepared uses ObjectDb::copy_record_from where the original save path uses the instance copy (copy_instance). copy_instance is test-only. | B4 |
| F253 | characters-06 | characters | CharacterRecord::duplicate (write then read back) has no live caller. Live code copies records with Clone, so the round trip's normalisation (for example of non-canonical voice names) is skipped. | B4 |
| F254 | characters-07 | characters | exa_game::item_effects::WornEffects::run passes lit=false and 0.0 to step_worn and drops the WornStep and ApparelStep results. Worn-item light and garment switches/dc0/1110/1190) never take effect, and fade_material has no live caller. | B4 |
| F255 | characters-07 | characters | run_worn_effects ignores EffectRun.flash, so the healing charge never shows the player's HUD for 1000 ms. | B4 |
| F256 | characters-07 | characters | exa_character::blood::paint draws its wound edge from a fixed xorshift WoundRng and works g and the value in f32. The original uses the game's Random and computes in double. | B4 |
| F257 | characters-07 | characters | exa_formats::rsq::sort_by_abs_x is never applied in Sequences::parse. Skeleton children keep the file's order instead of the original's sort by absolute x. | B4 |
| F258 | characters-07 | characters | No lookup by record id for the Command Dead and Loyalty re-link after a load; their control is always fed None. | B4 |
| F259 | characters-07 | characters | The perception term for status marker 0x102 (seen body with flag 0x10000, factors/ is absent from exa_ai. | B4 |
| F260 | characters-07 | characters | The shock release in combat does not reset the swing, and a grab is not released. | B4 |
| F261 | characters-07 | characters | exa_anim::bind_skins numbers joints by the bone number in node names, not in registration order. The 0x80000000 node-frame reset branch of has no counterpart. | B4 |
| F262 | characters-08 | characters | exa_ai::behaviour (BehaviourBase::attach/notify, MotileBehaviour trait) has no implementor or caller outside tests. No live controller uses the shared base: event counts, attach clears and the shared reset are absent in play. | B4 |
| F263 | characters-08 | characters | Keyframe tracks (exa_anim::tracks) start only from the F7 debug key. The original starts an object's tracks when the object is used, and that is not wired. | B4 |
| F264 | items-01 | items | Pack moves are probably lost: in panel_live's drop, a drop on the same panel returns before any write to Possessions, and save_list is never called. So moving an item inside the player's pack likely snaps back when the pack is opened again, because bind_pack reloads it from the record. Not confirmed with a run | B4 |
| F265 | items-01 | items | A container popup never clamps its entries after loading the record. The original calls after; Exarust's open_container_popup does not, so a child stored outside the panel stays outside it | B4 |
| F266 | items-01 | items | Item panel hit test: panel_live::entry_at passes alpha=true, so the icon-pixel test and the second, looser pass are missing. Clicks on transparent icon corners hit the item | B4 |
| F267 | items-01 | items | item_card::request_card ignores PanelEvent::ShowCard.run_use, so a right press on a use-kind-2 item never runs the game's use before its card | B4 |
| F268 | items-01 | items | Dropping the AR crystal (item 499) into a panel never runs the game function: DropResult.fired is unused | B4 |
| F269 | items-01 | items | Card builder fixes the rank and points gates off and the value gate on at build_for, and the pronoun sex is always male (female=false) | B4 |
| F270 | items-01 | items | Centre adds (doll undress, socket give-back, shop) never run the place pass, so returned items stack at the panel centre | B4 |
| F271 | items-01 | items | Pack to hand: equip_dropped removes the pack entry and re-adds it held at the centre instead of marking it held at its own cell | B4 |
| F272 | items-01 | items | Container::rescale (slot 22) has no live caller, so entry places are not rescaled when the interface unit changes | B4 |
| F273 | items-01 | items | figure_pick removes the held entry by item id; the original matches it by icon slot | B4 |
| F274 | items-01 | items | The worn panel layout seeds DOLL_SEED on every open instead of the shared engine generator (marked [stand-in], with no DEC entry) | B4 |

Stale documentation found (fix with the next change to each file):
the terrain module's activation names; `tree.rs` module doc ("nothing offers
a power yet"); the GI texture cache and `follow_camera` comments; the
RFE_MeshTools "not wired" rating; several pack ratings that predate round 146.

# Proposal

## Why

The sweep-render lane (Phase 5 — Visual and audio fidelity, see
docs/PLAN.md) swept 22 small rendering components: the free camera's
rules, the material loader's shared colour and transparency flag, the
immediate-draw format switches and program slots, the light's update
colours, the sun map's matrices, the texture filter modes, the material
manager's presence probes, the light alias weight, the scene fit's camera
node, the renderer's list choice and shader factory, the noise row fill,
the post variant and bloom kernel, the spline uniforms, the 3D view's
constructor words and the voxel light's constructor. Its review found
rules ported from pseudo-code or invented: the alias spot term read as
cosine and arc cosine where the engine calls its fast logarithm and
exponential, the face threshold taken from a flags word, the light update
gated by the body's flicker bits instead of the node's flags word, a
transposed light view matrix with the opposite cross product, a
caster-sphere sun fit and an interleaved vertex layout the original does
not have, a byte halving read from the octave pass, a spline width term
moved to single precision against the assembly, a wrong tether sign and
key bias, blue-first luma sums, and duplicated helpers (sphere, box and
segment tests, the corner merge, the push-out, the luma).

Evidence: REA (Ghidra) reads and the objdump listing of the free camera's
tick, its lift and wall-probe walks, the engine's sign, sphere, segment
and oriented-box tests, the material loader, the immediate-draw format
switch, the light's derive, update, clone and constructor, the alias's
reach tests and face mask, the fast logarithm and exponential, the light
activation's spot constants, the sun map's view and orthographic builders,
the sphere solver's callers, the scene fit, the runtime sine, the texture
filter install, the material manager's queries, the renderer's list
lookup and factory, the noise row fill, the bloom filter and kernel, the
post variant, the spline setup and the 3D view constructor.

## What Changes

- `exa_game::nav_camera`: the free camera's key vector, tether, drift,
  zoom spring and snap, and the lift and wall-probe scene walks on the
  engine's intersection tests (tested models).
- `exa_world::light_source`: `LightSource::update` (both workflows, the
  node-word gates, the multiplier, the running sum and its luma, the
  light node's colour and the reach's rebuild flag), beside main's
  `DerivedLight`.
- `exa_world::light_alias` (new): the plain and gated reach, the spot
  shape, the face mask and the mask merge (tested model).
- `exa_render`: `sun_fit::light_view_matrix` and `orthographic_matrix`,
  `passes::material_list` and `factory_list`,
  `shader_program::shader_word_for_level`, `post::post_variant`,
  `bloom_weights` and `bloom_kernel`, `splines::line_uniforms`,
  `quads::format_switches` and `BuiltinPrograms`,
  `material_state::draws_translucent`, `image_noise::white_noise_float_rows`,
  `gi::VoxelLight::constructed`, the 3D view's flag and texture words.
- `exa_formats::material::Material::shared_colour`;
  `exa_assets::material_manager` presence probes;
  `exa_items::item_renderer::frame_distance` divides by the sine and
  `frame_node` places the fit's node.
- `exa_ui::gui_draw` builds its quad triangles from the batcher's index
  template.

## Out of scope

The lane's directional caster cut (masked surfaces built outside the
record loader would lose their shadow cut) and its copy-before-upload in
the texture loader (a second copy of every texture for the same image)
were reverted. No new rule is wired into the running renderer except the
scene fit's sine (the item icon camera) and the GUI's index template; the
free camera, the light update and alias weight, the sun matrices, the
factory and post models have no caller yet.

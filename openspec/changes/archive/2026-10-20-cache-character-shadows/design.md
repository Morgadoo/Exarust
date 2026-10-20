# Design

`CubeCasters(EntityHashMap<u8>)` replaces the static list: caster → face
mask, 0 while unknown (drawn into every face, and treated as every face
when it is dirtied). `sphere_faces(light, centre, radius)` sets bit f when
the sphere reaches past the four side planes of Bevy's face f and in front
of the light; a sphere around the light touches all six.

`apply_relinks` inserts a newly linked caster with its mask and marks those
faces; an unlinked caster marks its old mask. `dirty_moved_casters`, after
the linking, takes every caster whose transform or box changed and, in each
light it is linked to, marks old | new and stores new.

`build_jobs` skips casters whose mask lacks the face. A skinned caster's
joints are written once a frame into `CubeJobs::joints` (joint
`GlobalTransform` × inverse bind pose) and its draw carries their offset;
the pipeline key gains `skinned`, which adds the joint index and weight
attributes (locations 2, 3) and the `SKINNED` shader def. The draw bind
group gains the joint buffer (an identity when nothing is skinned).

Lights never enable Bevy's shadow maps; meshes all carry render layer 0.

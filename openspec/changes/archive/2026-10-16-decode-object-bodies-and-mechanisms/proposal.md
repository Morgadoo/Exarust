## Why

In the original every prop, item, door, lever and pressure plate is a
four-particle rigid body built from a chunk its model carries, and the
object database records that place those models say which of them are
doors, levers or push surfaces and whom they signal. Exarust only knew
that a collision body followed a mesh: props are static trimesh colliders,
doors are baked into the level, and nothing reads the mechanisms. Before
props can join the 83 Hz solver (M32), hang from the world (M33), be dragged
(M34), or open, turn and press (M12, M35), and before held weapons can be
bodies with their own spheres (M24), the data has to be decoded.

Phase 3 (Bodies: skeleton, physics, control), feeding milestones M3
"standing man" and M4 "first duel"; master plan lane E, change M31
`decode-object-bodies-and-mechanisms` (milestones MS3 weapons as bodies
and MS4 working dungeon). Data only: nothing is simulated yet.

## What Changes

- A new `exa_formats::rigid_body` module, Bevy-free and total: the
  rigid-body chunk after a mesh body (class word, mobility, rest
  tetrahedron, point masses, the five words the original skips, then the
  collision spheres, the links with a slider's axis, and the scrape and
  roll sound slots), lengths in metres; a slider's travel is the square
  root of its stored slack.
- `rfc::Node` keeps the decoded body (or its error) instead of a flag,
  and whether the node carries the cloth marker chunk.
- `objdb::Item` reads the door section (flags, linked object, sound
  letters) and the operative section (class, kind, flags, value, return
  time, two targets), names the door sounds, and says which mechanism a
  record makes of its model (lever and push surface by operative class
  before door).
- `exa-tool bodies`, `exa-tool mechanisms` and `exa-tool corpus bodies`
  print them.
- Docs: `docs/formats/RFC.md` (Rigid body, Cloth marker),
  `docs/formats/OTHER.md` (Door and operative sections),
  `docs/formats/README.md`, `docs/PLAN.md`, crate READMEs.

Evidence:
- REA on `Exanima.exe` 0.9.5: the rigid-body chunk's reader and writer,
  the object factory that picks the body class from a record's sections,
  the door, lever and push-surface set-ups that read the sections, the
  signal dispatch, relays and jobs, the link conversion at registration,
  the cloth marker's handler and the game scene's set-up, the door sound
  names' strings (`context/re/master-physics.md` §1.1, §3.1, §4, §5, §7,
  §8.1; corrections appended there).
- Corpus: every chunk tree of `Objects.rpk`, `Objlib.rpk`,
  `Characters.rpk`, `CharExt.rpk` and `Resource.rpk`; the door and
  operative sections of `objdb.rdb` and of every level's own database.

Out of scope:
- simulating anything: props in the solver, gravity weights, material
  scaling of the mobility, registration asleep, articulated children and
  origin pivots (M32); world-link laws and the slider law (M33); the hand's
  drag and click classes (M34); doors, levers, plates and signals (M12,
  M35); weapons as bodies (M24); cloth (M37);
- the material's physical record the mobility is divided by (RB32), and
  the other object-database sections (`0x200`, `0x20000`; RB34);
- the meaning of class bits `0x400`, `0x8000` and `0x8`, link flag 2, the
  door flag `0x200`'s node flag, and the operative classes 0, 2, 4 and 8.

## Capabilities

### New Capabilities
- `object-physics`: rigid bodies of models, the cloth marker, door and
  operative records and the body class a record gives its model.

### Modified Capabilities
- `asset-inspection-cli`: commands that print bodies and mechanism records.

## Impact

- `exa_formats` (new `rigid_body`; `rfc::Node::rigid_body`, `cloth_marker`,
  `has_rigid_body`, `rigid`; `objdb::Item::door`, `operative`,
  `mechanism`, `door_sound`; `weapon_record` reads the new accessor)
- `exa_tool` (new `bodies` module and commands)
- `exa_character`, `exa_world` (test items gain the two fields)
- `docs/formats/RFC.md`, `docs/formats/OTHER.md`, `docs/formats/README.md`,
  `docs/PLAN.md`, `crates/exa_formats/README.md`, `crates/exa_tool/README.md`

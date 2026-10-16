## Context

Formats page: [`docs/formats/RFC.md`](../../../docs/formats/RFC.md) (Rigid
body, Cloth marker) and [`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md)
(Door and operative sections).

What the original reads (offsets from the chunk or section body):

1. **Rigid-body chunk** after a mesh body: u32 class, f32 mobility, four
   rest points (f32[3], cm), four point masses, then five words it skips
   (friction 0.75 in every body, 1.0, 0, a group mask it forces to 0, one
   more value). Sub-chunks to the end of the chunk, each u32 tag, u32 size:
   spheres (a skipped word, a count, count × centre and radius in cm),
   links (a count, then flags, the point on the body, the point on the
   parent, slack, strength, smoothing, and an axis when flag 4 marks a
   slider), sound slots (a count, then id, point, a 16-byte sound name,
   volume, pitch). Unknown tags are skipped.
2. **Link conversion** at registration: points and slack × the body's
   scale; a root body's links stay world links with flag `0x10000`
   cleared; a child body's keep only flag 8 and gain `0x10000`; a
   slider's fraction is its projected offset divided by the stored slack,
   which every shipped slider stores as |axis|² (the travel squared).
3. **Body class**: when an object's model is loaded, a record whose
   sections include the operative section gives the root body the lever
   class (operative class 3) or the push-surface class (5), or no special
   class (any other); otherwise a door section gives it the door class.
   Child bodies (parent node with a body) never take a special class.
4. **Door section** (20 bytes): flags (latches 2, locked 4, not clickable
   8, motorised `0x10`, fast `0x40`, inverted `0x80`, frozen `0x100`, a
   node flag `0x200`), the linked object (the key compared with the held
   one; relays of kind 1 signal the doors linked to them), four sound
   letters (lock, slam, hinge, unlatch). Sound names: `door`, `a` +
   letter, `_`, suffix (`LkO`/`LkT`, `slm`, `hng`, `opn`).
5. **Operative section** (60 bytes): class and kind bytes, u16 flags, u16
   value (a fire's size, a relay's state, an input's bit), u16 return time
   × 100 ms (levers), u32 target, u32 second target or relay code.
   Classes: 1 key, 3 lever (kind 6 fades a light), 5 push surface (kind 2
   inverted), 6 job (kind `0xC` fire, `0xD` light), 7 relay (kinds 0, 1,
   2, 3, 5).
6. **Cloth marker** (one word, 0 in every file): the node gets a cloth
   controller when the scene it loads into has flag `0x10000`, set on the
   game's scene when it is created.

The research notes, with addresses, are git-ignored:
`context/re/master-physics.md` §1.1, §3.1, §4, §5, §7, §8.1, and the
master plan's entry for M31; this change appends its corrections there.

## Goals / Non-Goals

**Goals:** decode every rigid body, cloth marker, door and operative
record of the installed data into plain Bevy-free types, total on
malformed input; print them with `exa-tool`; pin the corpus counts.

**Non-Goals:** any simulation or game use (M32–M37, M12, M24); the
material record the mobility is divided by; other object sections.

## Decisions

- **Metres, travel for sliders.** Lengths are stored in metres like every
  other `exa_formats` geometry. A slider's slack is stored as the travel
  (√slack), the axis carries the same length; the original's fraction
  (offset · axis / slack) is offset · axis / travel², so nothing is lost.
  A negative stored slack would be no travel (none ship).
- **`Node::rigid_body` keeps the error.** `Option<Result<RigidBody,
  String>>`, as materials keep theirs: a node that carries the chunk still
  counts as a rigid part for the weapon length walk (`has_rigid_body`)
  even if it fails to decode; every shipped body decodes.
- **Raw class, kind and flags.** Only the meanings the original's code
  establishes are named (constants in `objdb::door`, `objdb::operative`,
  `rigid_body::link`); the rest stay numbers.
- **The door's "key" is a linked object.** The field is called `key`
  after its main use, documented as the object the door is linked to:
  keys, relays and levers in `exanima01`.
- **Mechanism by sections, not by the index flags.** The original tests
  the database index's flag word, which is the OR of the section tags for
  every record but one; the parsed sections give the same answer.

## Risks / Trade-offs

- [Cloth condition] The plan said a material flag; the handler reads the
  flag word of the scene the node's sector belongs to. Resolved by the
  disassembly (the node's sector, the sector's scene, the scene's flag
  word) and the game scene's set-up; in game every marked node is cloth.
- [Counts] The plan's "65 with links" counts models hung from the world;
  69 models carry links in all (two flails and two scales link only child
  bodies). Its "39 levers, 11 push surfaces" in `exanima01` count kind 1
  only; there are 40 and 12 lever and push-surface records.

## Open Questions

- Class bits `0x400`, `0x8000`, `0x8` and link flag 2: REA on the
  readers of the class word in the solver (M32) and on the link laws
  (M33).
- What the door flag `0x200`'s node flag `0x80` does: REA on the readers
  of node flag `0x80` (M12).
- The operative classes 0, 2, 4, 8 and the name field after the targets:
  REA on the interact path (outside the plan) when a change needs them.
- The material value the mobility is divided by (RB32): REA on the
  material's physical record (M32).

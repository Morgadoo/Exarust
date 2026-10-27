# Design

- The weapon part tree is a flat list (`WeaponMassPart`) walked parent
  before children, depth first from each root in list order, as the
  original's recursion over each part's child list. The rigid nodes of the
  weapon model are the parts; each one's parent is its nearest rigid
  ancestor. The initial mass is the reciprocal of the node's registered
  mobility, a labelled stand-in for the shape builder's word.
- The holder's size divides only the root's live mass word after the
  passes; its inverse word keeps the distributed value.
- The world-join bits are written only where `PartState` carries them; the
  child's second-word mark and the collision layers are not kept.
- The counter decay moves to the start of the limb-strike system so a
  count added this tick stays whole until the next tick's contacts.

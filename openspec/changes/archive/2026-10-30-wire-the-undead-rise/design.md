# Design

The original never drops a dead character's state: death turns the
muscles off and the dead leave the body tick, and the revive turns the
muscles back on over the race pointer and stats the character kept.
Exarust's go-limp instead drops the drive and removes the components the
character tick selects on, so the revive keeps those aside: the race and
stats on the ragdoll (captured where the drive goes, since the collapse's
death drops it before the go-limp system runs), the clip player and the
controller in a component. A revived body (one with a drive and still
keeping them) gets them back the same frame, whoever revived it.

The rule runs in the frame (`Update`), as the original's manager frame
does, chained after go-limp and before the restore. Out-of-scene bodies
read as the manager's dormant ones. The record's weapon slots are read
from the hands, since Exarust's record keeps a dropped weapon. The
controller is put under the pelvis because Exarust's capsule (which the
original lacks) stayed where the character fell.

Not reproduced: the glow fade (no live skin-material fade; logged), combat
mode left on an AI controller, the revive's node-turn copy and two unknown
byte writes, the loop sources, the attack reach refresh (computed on
demand). The player is left to the flow's revive.

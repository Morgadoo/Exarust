# Design

The collapse already releases the item bodies
(`release_collapsed`, the grip release). The missing half is the
holder's side of each drop. Two records answer "what is in the hands":
the hands component (moveset, hand contacts, the rise's fallback) and the
possessions record (the out set the revive tests). Both are emptied on the
tick a collapse starts (`Added<Collapse>`), keeping a shield, which the
came-loose hook skips and which stays on the off arm.

The hands are emptied after the collapse words: those read whether the
body was armed (an unarmed body's hands go back to their base), and an
armed body's drop is followed by the equip refresh, which the hands'
change triggers on the next tick.

The rise reads the possessions record when the body has one (the
original's test), else the hands. The collapse record is removed at the
rise so `Added<Collapse>` fires again at a later collapse and the dying
tick's release flag starts clear.

Open questions: the hook's shield test is an item flag; Exarust uses the
catalogue's shield section without a weapon section, the same split the
grip release uses (answered well enough by the held kinds; not
re-verified per item). No docs/formats page changes.

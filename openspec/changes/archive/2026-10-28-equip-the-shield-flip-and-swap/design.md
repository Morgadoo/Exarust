# Design

## Shield factors, not a standing guard

The original writes the shield's factors on the character at equip and
uses them when a blow strikes the shield's part. Exarust strikes limbs,
never a shield body, and its `Guard::Shield` sends every blow on the
fighter down the block path; setting it at equip would make shield holders
immune to flesh wounds and keep them from raising a weapon parry. The
factors therefore wait on `ShieldHold` until shields are struck as bodies.

## One flip per character

The flip progress lives on the character and every held item reads it;
the attach writes it from the main hand's item only. The off-hand item
takes the same `holds_flipped` value as the main weapon.

## Refresh

`refresh_holds` counts held items of kind Weapon for the holder and
measures the one in the first blade slot (else the first found). The item
height is the model's top, the same length the record uses; for a
multi-node weapon the original reads the root node's box (guess, labelled).
The parry gate is the item's soft-weld gate; the original's character
flag is not identified.

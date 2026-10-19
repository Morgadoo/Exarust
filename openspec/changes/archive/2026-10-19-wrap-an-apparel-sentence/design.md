# Design

## Sentence

`apparel_sentence` calls `pick_phrase` for the condition line. A name shorter than three letters is returned with the same roll and the same side. When `fixed` is set, `fixed_name` runs and the roll does not step.

A name that ends in `]` loses its last four letters. The byte two before `]` being `p`, and the byte just before `]` not being `c`, composes "They are ". The byte just before `]` being `c` composes "It is" with no space and skips the quality roll, including the material edit. Any other bracket composes "It is ". Quality and condition are the byte divided by 2.55, dropping the fraction. Both marks at most 50, or both above 50, join with ", but". Differing sides join with " and".

A name that does not end in `]` drops a leading "A ". The first `.` that is not the last character is removed, and the bytes after it are appended after the condition phrase. Quality is a raw phrase from the first 14 made rows. Condition is `pick_phrase` on flawless, then perfect when that adds nothing. An unbracketed sentence takes "An " for a, e, i, o, or u, and "A " otherwise.

## Material

The caller supplies one material name. Ids 1 through 8 use it. The side line becomes that name, a space, and the old side, and a lowercase first byte becomes uppercase. That prefix is not written into the description. A 0x10 with a usable material is replaced, together with the next byte, by the material name and a space. Any other 0x10 is deleted with no article.

## Open questions

- Retail phrase rows and material names are not copied. The caller passes synthetic rows and one material name.
- The Bevy render graph still does not issue the original GL calls for a shell upload.

## Docs

`docs/PLAN.md` and `docs/COMPONENT-MAP.md` rate `TGUIItemContainer` at 26%. The published headline stays 14.4% and 5.7%. The Items section stays 17%.

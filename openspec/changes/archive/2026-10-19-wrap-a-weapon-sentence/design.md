# Design

## Sentence

`item_sentence` calls `pick_phrase` for the condition line. The quality line is the same roll and range test, stored without a leading space or a dot. A name shorter than three letters is returned unchanged. A trailing dot is dropped. A `]` ending drops four letters. A `c` immediately before that bracket builds the stripped name plus "It is" and skips the quality line. The dash flag is set only when the name starts with `-`, which also selects the high and great tables. Otherwise the made table is used, then flawless, and perfect when flawless adds nothing.

Quality and condition divide the byte by the stored double 2.55 and drop the fraction. Both marks at most 50, or both above 50, join a bracketed name with ", but". The other pair joins with " and". An unbracketed sentence is the quality phrase, a space, and the name. Its first letter a, e, i, o, or u takes "An ". Any other first byte takes "A ".

## Open questions

- The fixed-id name edit is not in. The retail phrase tables are not copied.

## Docs

`docs/PLAN.md` and `docs/COMPONENT-MAP.md` rate `TGUIItemContainer` at 22%. The published headline stays 14.4% and 5.7%.

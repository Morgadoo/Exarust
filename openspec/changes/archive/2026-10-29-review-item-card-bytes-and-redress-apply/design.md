## Context

The lane's three commits were reviewed against the instructions of the card builder, both sentence builders and the equipment cluster's plan, apply and take-in routines.

## Decisions

- **Sentence source.** The builder leaves its body pointer null unless no weapon page was built, so the weapon page wins; the lane's reversal is undone and the existing requirement stands.
- **Byte edits.** `SetLength` with a negative length empties the string, so a short bracket clears the name. The splice is written as the result of the original's loops: prefix, word, then a space and the tail only when a byte follows the mark. The empty-word branch (terminator copied backwards) is kept for totality; the installed variant words are never empty.
- **Apply.** The plan stores both stowed ids; the apply tests both for zero after the attaches. The class-word test is the apply's own, made once; per-model marks differ by kind (weapon owner -1 on success, shield 0x7fffffff, plain light none), so the lane's single mark set was removed rather than kept wrong.
- **Sheets.** A resource that nothing reads costs start-up work for no behaviour; the crop stays as a function.

## Risks

None for play: the running card keeps the sentence choice it had on main; the doll apply has no live caller.

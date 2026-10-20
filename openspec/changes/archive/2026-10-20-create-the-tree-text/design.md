# Design

## Behaviour

The create routine writes the language word `0x6e65` (`e`,`n`), the flags word `0x4e4`, the kind word 1 and a zero marker, then resets: the count becomes 0, the offset array becomes one zero, the blob size becomes 4 and the blob one zero u32. That is one empty string. The parser reads the same four header words before the count, so a fresh chunk parses to the same shape.

## Model

`new_tree_text() -> TreeText`; `TreeText { language, flags, kind, marker, strings }`.

## Open questions

The flags and kind meanings beyond their values are unknown.

## Docs

`TThaumTree` is **20%**. The headline rises slightly.

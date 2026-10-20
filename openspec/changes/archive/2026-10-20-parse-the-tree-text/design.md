# Design

## Behaviour

The header is 16 bytes and its word at offset 12 is the count. The next `(count + 1) * 4` bytes are the offsets into the blob, followed by the blob's u32 size and the blob itself. Each offset names a u32 length and then that many bytes. The parser returns one string per offset, rejecting a short header, any out-of-range read, the blob size past the input, or a non-UTF-8 string.

## Model

`parse_tree_text(bytes) -> Option<TreeText { strings }>`.

## Open questions

The header's other three words are not used by the parser.

## Docs

`TThaumTree` is **18%**. The headline rises slightly.

## ADDED Requirements

### Requirement: Tree text chunk
The tree text chunk SHALL read a 16-byte header whose last word is the count, count + 1 little-endian u32 offsets, a u32 blob size, and the blob, each string a u32 length followed by its bytes. A short header, an out-of-range offset, a blob past the input, or invalid UTF-8 SHALL return nothing.

#### Scenario: Three strings
- **WHEN** the count is 2, the offsets are 0, 6 and 13, and the blob holds "ab", "cde" and the empty string
- **THEN** the parser returns those three strings in order
- **WHEN** the header is short or the blob size overruns
- **THEN** the parser returns nothing

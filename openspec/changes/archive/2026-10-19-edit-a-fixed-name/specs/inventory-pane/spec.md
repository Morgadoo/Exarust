# Spec Delta

## ADDED Requirements

### Requirement: Fixed name
When the id word is -1, a name shorter than three letters SHALL stay unchanged and the roll SHALL not step. A longer name SHALL delete a 0x10 byte and the byte after it, shortening by two. It SHALL add "An " or "A " only when the first byte is at least Z.

#### Scenario: A mark comes out and an article goes on
- **WHEN** "ab" is fixed, "Sword", "axe", "Zed", and "zed" are fixed, and the names "ab" plus a mark plus "xy", "ab" plus a mark plus "c", a mark plus "cde", and "Za" plus a mark plus "bc" are fixed
- **THEN** "ab" stays "ab" at the same roll, "Sword" stays "Sword", "axe" is "An axe", "Zed" is "A Zed", "zed" is "A zed", the middle mark becomes "An aby", the end mark becomes "An ab", the leading mark becomes "A de", and "Za" becomes "A Zac"

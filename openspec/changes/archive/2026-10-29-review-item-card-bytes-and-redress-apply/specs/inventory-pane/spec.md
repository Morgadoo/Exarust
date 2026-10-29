## ADDED Requirements

### Requirement: Card name byte edits
The weapon sentence SHALL edit the name as bytes: a bracketed name shorter than four bytes after the trailing dot SHALL become empty, and no edit SHALL split a multi-byte character into an error. The apparel splice SHALL put the material word over the 0x10 mark and a space over the byte after it, moving the tail right; a mark that is the last byte SHALL take the word and no space.

#### Scenario: Short brackets and a mark at the end
- **WHEN** "é]" is wrapped by the weapon sentence and "ab" plus a mark is spliced with "wool"
- **THEN** the sentence keeps no part of the name and ends with its condition phrase, and the splice gives "abwool"

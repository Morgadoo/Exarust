# Spec Delta

## ADDED Requirements

### Requirement: Tree text
A text chunk after the nodes SHALL store each string as a length word and the bytes that follow it, at that string's offset. A length below 1 SHALL be empty. A tree with no text chunk SHALL have no strings. A text chunk that ends after its first word SHALL be an error.

#### Scenario: Blast, an empty string, and a short chunk
- **WHEN** a tree whose text chunk holds Blast and a length of 0 is read, the same nodes with no text chunk are read, and a text chunk that ends after its first word is read
- **THEN** the first text is Blast, the second is empty, the bare tree has no texts, and the short chunk is an error

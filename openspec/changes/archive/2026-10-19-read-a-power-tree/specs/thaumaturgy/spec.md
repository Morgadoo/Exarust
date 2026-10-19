# Spec Delta

## ADDED Requirements

### Requirement: Power tree
A power tree SHALL refuse a file whose first word is not the power magic. The node count SHALL be the word at byte 52. Each node SHALL carry a power id, two more words, an x, a y, and 16-byte links. A link count below 1 SHALL store no links. Every node after the first SHALL subtract the first node's x and y. A file that ends before the count SHALL be an error.

#### Scenario: Two nodes and a bad file
- **WHEN** a tree with a node at 10, 20 and a node at 15, 25 with one link is read, a link count below 1 is read, a file with a zero first word is read, and a file that ends after the magic is read
- **THEN** the first node stays at 10, 20, the second is at 5, 5 with that link, the low link count stores none, and the other two reads are errors

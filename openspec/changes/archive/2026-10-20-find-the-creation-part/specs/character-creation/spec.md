## ADDED Requirements

### Requirement: Creation part search
Finding SHALL search a node's attachments only when its kind masked with `0x89` equals `0x89` and its wanted count is not negative. An attachment tagged `0xFCA0` SHALL match. Otherwise the search SHALL continue into the node's children, and a leaf without a match SHALL return false.

#### Scenario: Attachment, child and miss
- **WHEN** a node of kind `0x89` with wanted count 2 has attachments `0x100` and `0xFCA0`
- **THEN** the search is true
- **WHEN** the wanted count is −1 and the only attachment is `0xFCA0`
- **THEN** the search is false without children
- **WHEN** a child matches
- **THEN** the search is true
- **WHEN** no node or attachment matches
- **THEN** the search is false

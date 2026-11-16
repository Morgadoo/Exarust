## ADDED Requirements

### Requirement: Empty extension field
A name without a dot, a name ending in a dot and a name whose extension is longer than 16 bytes SHALL be dispatched alike, as a name without an extension.

#### Scenario: A trailing dot
- **WHEN** an image named `wall01n.` holds the image magic
- **THEN** it goes to the image reader as `wall01n` does

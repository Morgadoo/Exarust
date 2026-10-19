# Spec Delta

## ADDED Requirements

### Requirement: Skinned shell sections
A skinned shell SHALL copy a section only when its flags include bit 2. Each copied triangle SHALL keep its three indices in order, after the remap. A section without that bit SHALL be skipped. A hidden section that also has bit 2 SHALL still be copied. A range that ends before it starts SHALL copy nothing. A short index list or a missing remap SHALL be an error.

#### Scenario: A marked section and a plain one
- **WHEN** a section with bit 2, a section without it, and a hidden section that also has bit 2 are copied, a reversed triangle with bit 2 is copied, a range that ends early is copied, a short list is copied, and a missing remap is copied
- **THEN** the kept list is 14, 15, 16, 11, 12, 10, the plain section is absent, the reversed triangle stays reversed, the early range is empty, the short list is an error, and the missing remap is an error

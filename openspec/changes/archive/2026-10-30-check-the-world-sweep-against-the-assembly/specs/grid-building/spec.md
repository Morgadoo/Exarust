# Spec Delta

## ADDED Requirements

### Requirement: Set-piece marking channel
The set-piece loader SHALL mark transparent parts in the model's first channel of kind 0x100 (the second word's low 16 bits); without one it SHALL append a channel whose first word is the first channel's first word with bit 0x4000000, whose second word is 0x2b40100, which copies the first channel's third word and slot count and whose slots start with every byte set. A part SHALL be marked when its alpha is transparent and its last triangle is not before its first.

#### Scenario: Reuse
- **WHEN** a model already loaded once is loaded again
- **THEN** its existing marking channel is found and no second one is added

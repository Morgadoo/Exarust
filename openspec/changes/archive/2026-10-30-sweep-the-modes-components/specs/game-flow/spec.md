# Spec Delta

## ADDED Requirements

### Requirement: Applied tip and discovery steps
The running app SHALL apply `Step::Tip` by showing the popup's choice in the tip banner, SHALL hide the banner on `Step::HideEndMenu` and `Step::ClosePrelude`, and SHALL turn `Step::Discovery` into a discovery pass at the game clock.

#### Scenario: A practice round's tip is hidden at the next fight
- **WHEN** a practice round ends with tip 0 and the next fight's start closes the end menu and the prelude
- **THEN** the banner shows a tip and is then empty

## ADDED Requirements

### Requirement: Trade window
The trade window SHALL be a 0x36c × 0x244 page titled "Trade" with the character preview and the player's pack, the quartered sell basket, the full-price buy basket and the stock at the page's right edge. Its bottom row SHALL show "Pay:" and the difference or "Receive:" and its negation; a purse short of the payment SHALL hide the label, warn "You cannot afford this transaction." and disable the Trade button, which paying with both baskets empty SHALL also disable.

#### Scenario: A short purse
- **WHEN** the buy basket's total less a quarter of the sell basket's exceeds the purse
- **THEN** the warning shows and the Trade button is disabled

# Spec Delta

## ADDED Requirements

### Requirement: Shop trade
The shop SHALL refuse two empty baskets, and SHALL refuse a buy total that exceeds the sell total by more than the company's copper. A confirmed trade SHALL change the copper by that difference, recording a payment as spent and a receipt as earned.

#### Scenario: Pay and receive
- **WHEN** a company with 343 copper is offered an empty trade, a buy of 400, a buy of 40, then a buy of 10 against a sell of 25
- **THEN** the empty trade and the buy of 400 are refused, the buy of 40 leaves 303 copper and 40 spent, and the last trade leaves 318 copper and 15 earned

# Design

## Context

The company holds copper. Nothing moves it for a shop basket.

## Goals / Non-Goals

**Goals:**

- Refuse an empty or unaffordable trade, and apply a net payment or receipt.

**Non-Goals:**

- What an item costs, and the shop window.

## Decisions

1. The amount due is the buy total minus the sell total. A larger sell is received.
2. Equal non-empty baskets confirm and leave the copper where it is.

## Risks / Trade-offs

- Item prices are not in this rule. The caller passes copper totals.

## Migration Plan

No data migration.

## Open Questions

The price of a single item was not read.

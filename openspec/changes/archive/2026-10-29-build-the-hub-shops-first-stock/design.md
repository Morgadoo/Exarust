# Design

The stock rules (`trade::update_stock`) already read a `ShopWorld`. The
running game supplies one as a Bevy resource, `LiveShopStock`, filled on the
first shop click from the resource pack (parse once; a missing or broken
database is remembered so it is not retried). Each table record becomes a
`ShopEntry` with the table's flags word; `TradeRecord::parse` gives the shop
fields.

Copies: the original makes a new item body for apparel (one quality draw
from the engine's shared seed) and for weapons (the item creator with its
variant). Exarust has no item-body database yet, so a copy is an id with
the instance bits and a counter, remembering its source for the shelf's
duplicate weighting and names; the seed steps once per copy (the weapon's
count is a guess). These are labelled in the code.

The merchant's stock type and specialisation live on `ArenaCareer` and go
through the save's merchant chunk in both directions; the click passes
them to `trade::ensure_first_stock_of`.

The side reads the node entity that carries the name: `spawn_model_nodes`
puts `Name` and `PartBounds` on the same entity, so the world centre is
`GlobalTransform::transform_point(centre)`. A node without bounds falls
back to its origin.

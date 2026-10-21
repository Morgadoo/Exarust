# Design

## Behaviour

The item-effect glow level is clamped first: above 0x40 it drops by four, otherwise it is 0x3c; then `0.5·(0.01·level)^0.7`. This is the original's `log`-then-`exp` chain with the 0.7 exponent and the final 0.5. Charge accumulates as `current + source·0.004`, clamped at the source, and the worn-part glow is 1 only for an odd kind below 0xff. A keyed entry list attaches an entry only when its key is absent.

The golem's dormancy scales multiply the stored values by 0.05 and 0.02 in double precision. Its homeward delay is the distance plus π plus a roll centred on 0.5, and its reach is 200 plus the stored offset.

The electric arc's colour is `(0, 53, 0)`. Parts are filtered by their 5-bit kind and sub-kind byte. The spark brightness is `0.8 + 0.2·roll`. Releasing an electrified weapon hides a 0x21-kind node, sets its state to 1 and clears the node's two callbacks, whether or not the node matched.

## Model

`exa_game::item_effects` (charge, glow, entry list), `exa_ai::golem` (fades, delay, reach) and `exa_render::electricity` (colour, filters, brightness, release).

## Open questions

The arc mesh, ribbon buffers, spark burst, golem wake/alarm handlers and item-effect dispatch are separate and remain open.

## Docs

The three components are **20%**, **20%** and **10%**. The headline rises slightly.

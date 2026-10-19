# Design

## Accept

`Pane::accept`, which `drop_icon` calls, refuses a shared reject bit first. It then refuses a container dropped onto itself, a size above a positive limit, and a bulk that passes a positive capacity from outside. The required mask must then match. An own-only pane accepts only an id already in the pane whose low flag bit is set. A limit of 0 and a capacity of 0 skip those two checks.

## Open questions

- The icon picture and the description card are not in.

## Docs

`docs/PLAN.md` and `docs/COMPONENT-MAP.md` rate `TGUIItemContainer` at 5%. The published headline stays 14.4% and 5.7%.

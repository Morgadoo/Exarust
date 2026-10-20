# Design

## Behaviour

The first test is the query against the signature's id at `(%rbx)`; equality matches. Otherwise the binding is looked up by the query. With no binding the match is false. The binding's word at `+0x14` must equal the signature's id; then each of four bytes at `+0x1bc + i × 12` must equal the signature byte at `+0xf + i`. All four equal is a match.

## Model

`Signature` carries the id, the four bytes and an optional `Binding` with its id and parts. `signature_matches(query, signature)` reproduces the two paths.

## Open questions

The lookup is not in.

## Docs

This method is 141 bytes. `SG_Character_Create` stays about 10% (about 10.5% by bytes). The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

# Design

## Behaviour

The first test is the query against the signature's id; equality matches. Otherwise the binding is looked up by the query. With no binding the match is false. The binding's id word must equal the signature's id; then each of the record's four signature bytes (one per 12-byte entry) must equal the matching signature byte. All four equal is a match.

## Model

`Signature` carries the id, the four bytes and an optional `Binding` with its id and parts. `signature_matches(query, signature)` reproduces the two paths.

## Open questions

The lookup is not in.

## Docs

This method is 141 bytes. `SG_Character_Create` stays about 10% (about 10.5% by bytes). The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

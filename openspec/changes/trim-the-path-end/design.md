# Design

## Behaviour

The threshold is `64/(quality+1)`, with the quality floored at 0. Starting at the path's end, the trim drops a point whose penalty byte exceeds the threshold, at most eight times, and never below one point. It returns the remaining count.

## Model

`trim_path_end(points, penalties, quality) -> usize`.

## Open questions

The greedy line-walk budget in the same reconstruction is separate.

## Docs

`TPathFinder` is **60%**. The headline stays 14.4% and 5.7%.

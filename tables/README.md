# Opening Tables

Place external opening-book TSV files in this folder.

The engine loader (`src/tables/opening_book.rs`) checks these paths at startup:

1. `tables/lichess_openings.tsv`
2. `tables/openings.tsv`
3. `tables/chess-openings.tsv`

Expected format:

- Tab-separated text file with a header row.
- Must contain either a `uci` column (preferred) or a `moves` column.
- Optional weight columns supported: `weight`, `count`, or `plays`.

Example header:

```
eco	name	uci	weight
```

If no external file exists, the engine falls back to an embedded minimal book.

## Import from lichess-org/chess-openings

Once network access is available, you can clone/download:

`https://github.com/lichess-org/chess-openings`

Then transform/copy the relevant TSV into one of the filenames above, ensuring
it has a `uci` (or `moves`) column.

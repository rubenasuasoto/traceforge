# Synthetic datasets

- `sample-mixed-4k.jsonl`: 4,000 deterministic events, seed 42, mixed scenario.
- `sample-mixed-250.csv`: the same contract demonstrated in CSV at a smaller scale.

All values are invented. Addresses in public-looking ranges use RFC 5737 documentation networks. Regenerate either file with the `traceforge generate` command; byte-for-byte event content is stable for the same version, seed, count and scenario.

`.tfi` files are generated index payloads and are intentionally ignored. Build one locally with:

```text
traceforge build-index --input datasets/sample-mixed-4k.jsonl --output sample.tfi
```


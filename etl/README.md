# maplestory-union-solver/etl

ETL toolchain: solver JSONL trace shards → Parquet → Hugging Face Hub.

The artifact (the Parquet dataset) is published separately on Hugging
Face Hub under CC-BY-4.0. This directory only handles the conversion and
upload.

## Setup

```bash
cd etl
uv sync
```

Depends on `pyarrow` and `huggingface_hub`. About 100 MB venv.

## JSONL → Parquet

Convert per chunk:

```bash
uv run python jsonl_to_parquet.py \
    --in  ../shards/chunk-00 \
    --out ../parquet/chunk-00
```

Outputs:
- `../parquet/chunk-00/instances.parquet`
- `../parquet/chunk-00/branches.parquet`

For all 10 chunks, run the same command 10 times (`chunk-01..09`).
Each chunk takes 1-2 minutes to convert.

## HF Hub push

```bash
uv run huggingface-cli login
uv run huggingface-cli upload <user>/<dataset> \
    ../parquet/ . \
    --repo-type=dataset
```

The dataset repo name and HF Hub account are decided at push time.

## Schema documentation

The Parquet schema is defined in `jsonl_to_parquet.py` as top-level
constants (`INSTANCES_SCHEMA`, `BRANCHES_SCHEMA`). These are the
implementation source of truth until the dataset is first published —
after that, the dataset card README on the Hugging Face Hub becomes the
canonical schema documentation for downstream consumers.

## License

MIT. See `LICENSE`.
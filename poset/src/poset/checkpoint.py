# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 CloudHolic

"""Internal checkpoint loading utilities."""

import json
from pathlib import Path

from huggingface_hub import snapshot_download

_MODEL_TYPE = "poset"


def load_checkpoint(
    name_or_path: str | Path,
    revision: str | None = None,
    token: str | None = None,
) -> Path:
    """Resolve a checkpoint location and load its validated config."""

    path = Path(name_or_path)
    if path.is_dir():
        local_path = path
    else:
        local_path = Path(snapshot_download(
            repo_id=str(name_or_path),
            revision=revision,
            token=token
        ))

    config = json.loads((local_path / "config.json").read_text())
    model_type = config.get("model_type", None)
    if model_type != _MODEL_TYPE:
        raise ValueError(
            f"checkpoint at {name_or_path!r} is not a POSET model "
            f"(model_type: {model_type!r})."
        )

    return local_path
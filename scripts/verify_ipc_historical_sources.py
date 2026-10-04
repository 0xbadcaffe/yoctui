"""Validate retained IPC sources at their recorded revision plus source patch."""

from pathlib import Path, PurePosixPath
import hashlib
import re
import subprocess
import tempfile


def verify_sources(manifest: dict, root: Path = Path(".")) -> None:
    root = root.resolve()
    revision = manifest.get("source_base_revision", "")
    if not re.fullmatch(r"[0-9a-f]{40}", revision):
        raise ValueError("IPC historical source revision is invalid")
    sources = manifest.get("sources", {})
    patches = [source for source in sources if source.endswith(".patch")]
    if len(patches) != 1:
        raise ValueError("IPC historical sources need exactly one recorded patch")
    for source, digest in sources.items():
        path = PurePosixPath(source)
        if path.is_absolute() or ".." in path.parts or not re.fullmatch(r"[0-9a-f]{64}", digest):
            raise ValueError("IPC historical source path or digest is invalid")
    patch_path = root / patches[0]
    if hashlib.sha256(patch_path.read_bytes()).hexdigest() != sources[patches[0]]:
        raise ValueError("IPC historical source patch digest mismatch")
    subprocess.run(
        ["git", "merge-base", "--is-ancestor", revision, "HEAD"], cwd=root,
        check=True, capture_output=True,
    )
    # Reconstruct only in an owned temporary tree. Never apply old patches to
    # the current checkout or relabel historical measurements as fresh results.
    with tempfile.TemporaryDirectory(prefix="yoctui-ipc-source-proof-") as directory:
        tree = Path(directory)
        subprocess.run(["git", "init", "-q", directory], check=True, capture_output=True)
        for source in sources:
            if source in patches:
                continue
            original = subprocess.check_output(["git", "show", f"{revision}:{source}"], cwd=root)
            destination = tree / source
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(original)
        subprocess.run(
            ["git", "-C", directory, "apply", "--unidiff-zero", str(patch_path)],
            check=True, capture_output=True,
        )
        for source, digest in sources.items():
            if source in patches:
                continue
            if hashlib.sha256((tree / source).read_bytes()).hexdigest() != digest:
                raise ValueError(f"IPC historical source digest mismatch: {source}")

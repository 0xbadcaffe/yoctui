"""Strict static checks for the bridge's actual ordered shared namespace."""
import argparse
import ast
import re
import textwrap
from pathlib import Path
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parents[1]
BRIDGE = ROOT / "crates/yoctui-bitbake/bridge/yoctui_bridge.py"


def assemble(loader: Path = BRIDGE) -> tuple[str, list[Path]]:
    tree = ast.parse(loader.read_text())
    assignments = {
        node.targets[0].id: node.value
        for node in tree.body if isinstance(node, ast.Assign)
        and isinstance(node.targets[0], ast.Name)
    }
    names = ast.literal_eval(assignments["_COMPONENTS"])
    directory = loader.with_name("yoctui_bridge_components")
    if len(names) != len(set(names)) or set(names) != {path.name for path in directory.glob("*.py")}:
        raise ValueError("Bridge static checks must include every loader component exactly once")
    paths = [directory / name for name in names]
    source = "\n\n".join(path.read_text() for path in paths) + "\n"
    # Never execute the projection: only lint/type-check the identical namespace.
    ast.parse(source)
    return source, paths


def main() -> None:
    # Mode flags are deliberately the complete interface.
    parser = argparse.ArgumentParser()
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--lint", action="store_true")
    group.add_argument("--types", action="store_true")
    args = parser.parse_args()
    source, paths = assemble()
    with tempfile.TemporaryDirectory(prefix="yoctui-bridge-static-") as directory:
        assembled = Path(directory) / "assembled_yoctui_bridge.py"
        assembled.write_text(source if args.lint else type_projection(source))
        targets = [str(assembled), str(BRIDGE), str(ROOT / "bridge/tests")]
        command = ["ruff", "check", "--config", str(ROOT / "pyproject.toml")] if args.lint else ["mypy"]
        subprocess.run(command + targets, cwd=ROOT, check=True)
    print(f"Strict bridge {'lint' if args.lint else 'types'} passed: {len(paths)} ordered components, loader and tests")


def type_projection(source: str) -> str:
    """Represent exact runtime method bindings as class methods for mypy."""
    tree = ast.parse(source)
    classes = {node.name: node for node in tree.body if isinstance(node, ast.ClassDef)}
    functions = {node.name: node for node in tree.body if isinstance(node, ast.FunctionDef)}
    removed = set()
    additions = {}
    for node in tree.body:
        if isinstance(node, ast.Assign) and len(node.targets) == 1:
            target = node.targets[0]
            if isinstance(target, ast.Attribute) and isinstance(target.value, ast.Name) and target.value.id in classes:
                if not isinstance(node.value, ast.Name) or node.value.id not in functions:
                    raise ValueError("Unsupported dynamic bridge method binding")
                function = functions[node.value.id]
                if function.decorator_list:
                    raise ValueError("Unsupported decorated bridge method binding")
                body = ast.get_source_segment(source, function)
                body = re.sub(rf"^def {re.escape(function.name)}\(", f"def {target.attr}(", body, count=1)
                additions.setdefault(classes[target.value.id].end_lineno, []).append(textwrap.indent(body, "    "))
                removed.update(range(node.lineno, node.end_lineno + 1))
    # Retain original source comments, including its narrowly scoped optional
    # native-bb import directive. Only represent exact bound methods in class.
    lines = []
    for number, line in enumerate(source.splitlines(), 1):
        if number not in removed:
            lines.append(line)
        for method in additions.get(number, []):
            lines.extend(["", method])
    projected = "\n".join(lines) + "\n"
    ast.parse(projected)
    return projected


if __name__ == "__main__":
    main()

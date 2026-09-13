"""Render released signatures with source-maintained, reviewed API explanations.

Stable membership and types always come from the release tag. Explanations come
from the checkout stubs, where they also power editor hover help. Keep help for
shared members valid for both versions; describe new calling conventions in
signatures and the Next guide. Missing help is a build error.
"""

from __future__ import annotations

import argparse
import ast
import copy
import inspect
import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def source(root: Path, tag: str, path: str, channel: str) -> str:
    """Read released declarations from Git or maintained help from the checkout."""
    if channel == "stable":
        return subprocess.check_output(
            ["git", "show", f"{tag}:{path}"], cwd=root, text=True
        )
    return (root / path).read_text()


def name(node: ast.AST) -> str | None:
    if isinstance(node, (ast.ClassDef, ast.FunctionDef)):
        return node.name
    if isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name):
        return node.target.id
    return None


def documented_nodes(nodes: list[ast.stmt]) -> dict[str, tuple[ast.AST, str]]:
    """Index definitions and PEP 257 attribute docstrings without importing code."""
    result = {}
    for index, node in enumerate(nodes):
        key = name(node)
        if key is None:
            continue
        comment = ""
        if isinstance(node, (ast.ClassDef, ast.FunctionDef)):
            comment = ast.get_docstring(node) or ""
        elif index + 1 < len(nodes):
            following = nodes[index + 1]
            if (
                isinstance(following, ast.Expr)
                and isinstance(following.value, ast.Constant)
                and isinstance(following.value.value, str)
            ):
                comment = following.value.value
        result[key] = (node, comment)
    return result


def help_for(entries: dict, key: str, context: str) -> str:
    comment = inspect.cleandoc(entries.get(key, (None, ""))[1])
    if not comment or "Initialize self. See help(type(self))" in comment:
        raise ValueError(f"Missing source docstring: {context}.{key}")
    return comment


def signature(node: ast.FunctionDef, class_name: str | None = None) -> str:
    rendered = copy.deepcopy(node)
    rendered.body = [ast.Expr(ast.Constant(Ellipsis))]
    rendered.decorator_list = []
    if node.name == "__init__" and class_name:
        rendered.name = class_name
        rendered.args.args = rendered.args.args[1:]
        rendered.returns = None
    return ast.unparse(rendered).split(":\n", 1)[0].removeprefix("def ")


def write_page(
    path: Path, title: str, channel: str, tag: str, declaration: str, body: list[str]
):
    status = (
        "Stable " + tag.removeprefix("v")
        if channel == "stable"
        else "Next API · unreleased"
    )
    lines = [
        f'---\ntitle: "Python · {title}"\ndescription: "{title} signatures, defaults and API explanations."\naudience: user\npagefind: {str(channel == "stable").lower()}\n---',
        f"**{status}.** "
        + (
            "These signatures match the released Python package. Explanations are maintained in the source docstrings and reviewed against this release."
            if channel == "stable"
            else f"This reference describes the source checkout. Compare with stable rexafs {tag.removeprefix('v')} before using it with an installed package."
        ),
        "[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)",
        f"[Declaration source](https://github.com/Ameyanagi/rexafs/blob/{tag if channel == 'stable' else 'main'}/{declaration}) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/{declaration})",
        *body,
    ]
    text = "\n\n".join(lines)
    text = re.sub(r"(?<![<(])(https?://[^\s<>]+)(?=\s|$)", r"<\1>", text)
    path.write_text(text + "\n")


def generate(root: Path = ROOT):
    """Regenerate both channels from source; missing public help raises ValueError.

    ``root`` selects a checkout with its release tag available locally. Only the
    generated Python reference directory is replaced; installed packages and
    source declarations are never modified.
    """
    out = root / "website/src/content/docs/docs/reference"
    tag = json.loads((root / "website/src/data/release.json").read_text())["tag"]
    counts = {}
    for channel in ["stable", "next"]:
        channel_dir = out / channel / "python"
        channel_dir.mkdir(parents=True, exist_ok=True)
        for old in channel_dir.glob("*.md"):
            old.unlink()
        count = 0
        for relative, module_name in [
            ("__init__.pyi", "rexafs"),
            ("io.pyi", "rexafs.io"),
        ]:
            path = "py-rexafs/python/rexafs/" + relative
            tree = ast.parse(source(root, tag, path, channel))
            current = documented_nodes(ast.parse(source(root, tag, path, "next")).body)
            for node in tree.body:
                if not isinstance(
                    node, (ast.ClassDef, ast.FunctionDef)
                ) or node.name.startswith("_"):
                    continue
                title = node.name if module_name == "rexafs" else "io." + node.name
                body = [help_for(current, node.name, module_name)]
                if isinstance(node, ast.ClassDef):
                    entries = documented_nodes(current[node.name][0].body)
                    members = node.body
                else:
                    entries, members = current, [node]
                    body = []
                for member in members:
                    key = name(member)
                    if key is None or (key.startswith("_") and key != "__init__"):
                        continue
                    label = node.name if key == "__init__" else key
                    declaration = (
                        signature(member, node.name)
                        if isinstance(member, ast.FunctionDef)
                        else ast.unparse(member)
                    )
                    body += [
                        f"## {label}",
                        f"```python\n{declaration}\n```",
                        help_for(entries, key, title),
                    ]
                    count += 1
                write_page(
                    channel_dir / (title.lower().replace(".", "-") + ".md"),
                    title,
                    channel,
                    tag,
                    path,
                    body,
                )
            if module_name == "rexafs":
                body = []
                for node in tree.body:
                    if isinstance(node, ast.AnnAssign) and isinstance(
                        node.target, ast.Name
                    ):
                        key = node.target.id
                        body += [
                            f"## {key}",
                            f"```python\n{ast.unparse(node)}\n```",
                            help_for(current, key, module_name),
                        ]
                write_page(
                    channel_dir / "types.md", "types and version", channel, tag, path, body
                )
        counts[channel] = count
    print("Generated Python documented members:", counts)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root", type=Path, default=ROOT, help="Repository checkout to document."
    )
    generate(parser.parse_args().root.resolve())

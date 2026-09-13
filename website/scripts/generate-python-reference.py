"""Generate user API pages from the packaged stubs and Python docstrings."""

from __future__ import annotations
import ast
import importlib
import inspect
import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "website/src/content/docs/docs/reference"
TAG = json.loads((ROOT / "website/src/data/release.json").read_text())["tag"]


def source(path: str, channel: str) -> str:
    if channel == "stable":
        return subprocess.check_output(
            ["git", "show", f"{TAG}:{path}"], cwd=ROOT, text=True
        )
    return (ROOT / path).read_text()


def doc(obj: object) -> str:
    return inspect.getdoc(obj) or ""


def signature(node: ast.FunctionDef) -> str:
    copy = ast.FunctionDef(
        name=node.name,
        args=node.args,
        body=[ast.Expr(ast.Constant(Ellipsis))],
        decorator_list=[],
        returns=node.returns,
        type_comment=None,
        lineno=0,
    )
    return ast.unparse(copy).split(":\n", 1)[0].removeprefix("def ")


package = importlib.import_module("rexafs")
assert package.__version__ == TAG.removeprefix("v"), package.__version__
counts = {}
for channel in ["stable", "next"]:
    channel_dir = OUT / channel / "python"
    channel_dir.mkdir(parents=True, exist_ok=True)
    for old in channel_dir.glob("*.md"):
        old.unlink()
    count = 0
    for relative, module_name in [("__init__.pyi", "rexafs"), ("io.pyi", "rexafs.io")]:
        module = importlib.import_module(module_name)
        path = "py-rexafs/python/rexafs/" + relative
        tree = ast.parse(source(path, channel))
        for node in tree.body:
            if not isinstance(
                node, (ast.ClassDef, ast.FunctionDef)
            ) or node.name.startswith("_"):
                continue
            title = node.name if module_name == "rexafs" else "io." + node.name
            runtime = getattr(module, node.name, None) if channel == "stable" else None
            lines = [
                f'---\ntitle: "Python · {title}"\ndescription: "{title} signatures, types and docstrings."\naudience: user\npagefind: {str(channel == "stable").lower()}\n---\n',
                f"**{'Stable ' + TAG.removeprefix('v') if channel == 'stable' else 'Next API · unreleased'}.** "
                + (
                    "Install the stable package to use these signatures."
                    if channel == "stable"
                    else "These signatures describe the source checkout. They are not available from `pip install rexafs==0.2.4`."
                ),
                "\n[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)\n",
                ast.get_docstring(node) or (doc(runtime) if runtime else ""),
                f"\n[Declaration source](https://github.com/Ameyanagi/rexafs/blob/{TAG if channel == 'stable' else 'main'}/{path})\n",
            ]
            members = node.body if isinstance(node, ast.ClassDef) else [node]
            for index, member in enumerate(members):
                if isinstance(member, ast.FunctionDef) and (
                    not member.name.startswith("_") or member.name == "__init__"
                ):
                    label = node.name if member.name == "__init__" else member.name
                    lines += [
                        f"\n## {label}\n",
                        "```python\n" + signature(member) + "\n```",
                        ast.get_docstring(member)
                        or (
                            doc(getattr(runtime, member.name, None))
                            if runtime and isinstance(node, ast.ClassDef)
                            else ""
                        ),
                    ]
                    count += 1
                elif isinstance(member, ast.AnnAssign) and isinstance(
                    member.target, ast.Name
                ):
                    name = member.target.id
                    following = members[index + 1] if index + 1 < len(members) else None
                    comment = (
                        following.value.value
                        if isinstance(following, ast.Expr)
                        and isinstance(following.value, ast.Constant)
                        and isinstance(following.value.value, str)
                        else ""
                    )
                    if not comment and runtime:
                        comment = doc(getattr(runtime, name, None))
                    lines += [
                        f"\n## {name}\n",
                        "```python\n" + ast.unparse(member) + "\n```",
                        comment,
                    ]
                    count += 1
            # Make bare citation URLs in API docstrings clickable without inventing bibliography entries.
            text = "\n\n".join(part for part in lines if part)
            text = re.sub(r"(?<![<(])(https?://[^\s<>]+)(?=\s)", r"<\1>", text)
            (channel_dir / (title.lower().replace(".", "-") + ".md")).write_text(
                text + "\n"
            )
        aliases = [
            ast.unparse(n)
            for n in tree.body
            if isinstance(n, ast.AnnAssign) and isinstance(n.target, ast.Name)
        ]
        if module_name == "rexafs":
            (channel_dir / "types.md").write_text(
                f'---\ntitle: "Python · type aliases"\ndescription: "Literal types and module attributes."\naudience: user\npagefind: {str(channel == "stable").lower()}\n---\n\n**{channel} API**. See the [version guide](/docs/reference/).\n\n```python\n'
                + "\n\n".join(aliases)
                + "\n```\n"
            )
    counts[channel] = count
print("Generated Python documented members:", counts)

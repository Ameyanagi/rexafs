"""Build identities shared by packaging and macOS signing."""
import re


def app_name(channel):
    """Return the stable or nightly macOS bundle name; reject unknown channels.

    Separate names let the two applications coexist in Applications. This
    validates only the channel label, not a compiled application's identity.
    """
    if channel not in {"stable", "nightly"}:
        raise ValueError("Unknown desktop channel")
    return "rexafs Nightly.app" if channel == "nightly" else "rexafs.app"


def identity(version, environment):
    """Resolve build identity from an environment mapping without modifying it.

    Defaults are the stable channel and tag ``v{version}``; built_at is None
    unless REXAFS_BUILD_UTC is set. Nightly requires a nightly-YYYYMMDD-RUN_ID
    tag and a nonempty timestamp. This helper checks the tag's shape but does
    not parse the timestamp or verify the run; nightly-release.py supplies
    the run-derived UTC identity. Invalid channel/tag combinations raise
    ValueError.
    """
    channel = environment.get("REXAFS_BUILD_CHANNEL", "stable")
    app_name(channel)
    tag = environment.get("REXAFS_BUILD_TAG", "v" + version)
    if channel == "nightly":
        if not re.fullmatch(r"nightly-\d{8}-\d+", tag) or not environment.get("REXAFS_BUILD_UTC"):
            raise ValueError("Nightly builds require an immutable dated tag and build timestamp")
    elif tag != "v" + version:
        raise ValueError("Stable build tag must match the package version")
    return {"channel": channel, "release_tag": tag,
            "built_at": environment.get("REXAFS_BUILD_UTC")}

"""macOS application identity and Finder document associations for packaging."""

from desktop_channels import app_name


PROJECT_CONTENT_TYPE = "com.rexafs.project"


def info_plist(version, channel):
    """Return bundle metadata for a stable or nightly desktop application.

    Both channels open the same .rxs project format. Stable advertises ownership
    of that format; nightly remains an alternate editor when both are installed.
    The explicit content type maps the filename extension to Finder open events.
    See Apple's document-type and exported-type declarations:
    https://developer.apple.com/documentation/uniformtypeidentifiers/defining-file-and-data-types-for-your-app
    """
    display_name = app_name(channel).removesuffix(".app")
    return {
        "CFBundleExecutable": "rexafs",
        "CFBundleIdentifier": "com.rexafs.nightly" if channel == "nightly" else "com.rexafs.desktop",
        "CFBundleName": display_name,
        "CFBundleDisplayName": display_name,
        "CFBundlePackageType": "APPL",
        "CFBundleIconFile": "rexafs.icns",
        "CFBundleShortVersionString": version.split("-")[0],
        "CFBundleVersion": version.split("-")[0],
        "NSHighResolutionCapable": True,
        "CFBundleDocumentTypes": [{
            "CFBundleTypeName": "rexafs project",
            "CFBundleTypeExtensions": ["rxs"],
            "CFBundleTypeRole": "Editor",
            "CFBundleTypeIconFile": "rexafs.icns",
            "LSItemContentTypes": [PROJECT_CONTENT_TYPE],
            "LSHandlerRank": "Alternate" if channel == "nightly" else "Owner",
        }],
        "UTExportedTypeDeclarations": [{
            "UTTypeIdentifier": PROJECT_CONTENT_TYPE,
            "UTTypeDescription": "rexafs project",
            "UTTypeConformsTo": ["public.json"],
            "UTTypeTagSpecification": {"public.filename-extension": ["rxs"]},
        }],
    }

"""Check packaged Finder associations without building or registering an app."""

import plistlib
import unittest

from macos_bundle import PROJECT_CONTENT_TYPE, info_plist


class MacBundleTests(unittest.TestCase):
    def test_finder_association_matches_the_exported_project_extension(self):
        for channel in ("stable", "nightly"):
            with self.subTest(channel=channel):
                # Use the serialized payload consumed by Launch Services.
                metadata = plistlib.loads(plistlib.dumps(info_plist("0.3.0-rc.1", channel)))
                document, = metadata["CFBundleDocumentTypes"]
                exported, = metadata["UTExportedTypeDeclarations"]
                self.assertEqual(document["LSItemContentTypes"], [PROJECT_CONTENT_TYPE])
                self.assertEqual(document["CFBundleTypeRole"], "Editor")
                self.assertEqual(exported["UTTypeIdentifier"], PROJECT_CONTENT_TYPE)
                self.assertEqual(exported["UTTypeConformsTo"], ["public.json"])
                self.assertEqual(exported["UTTypeTagSpecification"]["public.filename-extension"], ["rxs"])
                self.assertEqual(document["CFBundleTypeExtensions"], ["rxs"])
                self.assertEqual(metadata["CFBundleVersion"], "0.3.0")

    def test_channels_keep_distinct_identities_and_stable_is_the_owner(self):
        stable = info_plist("0.3.0", "stable")
        nightly = info_plist("0.3.0", "nightly")
        self.assertEqual(stable["CFBundleIdentifier"], "com.rexafs.desktop")
        self.assertEqual(nightly["CFBundleIdentifier"], "com.rexafs.nightly")
        self.assertEqual(stable["CFBundleName"], "rexafs")
        self.assertEqual(nightly["CFBundleName"], "rexafs Nightly")
        self.assertEqual(stable["CFBundleDocumentTypes"][0]["LSHandlerRank"], "Owner")
        self.assertEqual(nightly["CFBundleDocumentTypes"][0]["LSHandlerRank"], "Alternate")
        with self.assertRaises(ValueError):
            info_plist("0.3.0", "unknown")


if __name__ == "__main__":
    unittest.main()

# rexafs 0.2.6 import screenshot provenance

This record identifies the signed application and retained measurement used for
the full-window import capture. The image was inspected through computer use;
it is not a mockup. Earlier captures retain their original versions.

```json
{
  "captured_on": "2026-09-14",
  "platform": "macOS ARM64",
  "version": "0.2.6",
  "method": "Computer-use getScreenshot; original full PNG with no cropping, resizing or compositing.",
  "width": 1192,
  "height": 768,
  "image": {
    "path": "website/public/screenshots/0.2.6/import-preview.png",
    "sha256": "68c462b9c021da0bd6d338062a704f9eaace7f06848e2c1358ea0e6f49ef3cf7"
  },
  "executable_sha256": "2afcc7f0865fba8b9cbb4849146fe6622cad51610c7e5645d192b3f722ab00b1",
  "input": {
    "path": "crates/rexafs/tests/fixtures/xas/samples/nsls-ii/7-bm-qas/xasref/Mo foil 0001-r0003.dat",
    "sha256": "280452e66ffd87d41bfa0d3609e51512aa9264e66bed65ad0fb4942cf20bb135",
    "source_url": "https://github.com/Ameyanagi/xasref/blob/74d1e795855055c7731da406b276bd50b27aafff/foil_QAS_sample_position/Mo%20foil%200001-r0003.dat",
    "attribution": "Ryuichi Shimogawa and contributors. xasref: reference XAS spectra.",
    "license_basis": "MIT repository distribution notice; original adjacent .license attribution retained."
  },
  "observed": "Transmission, fluorescence and reference are checked; Previewing Reference displays ln(it / ir); Import 3 spectra remains enabled.",
  "source_commit": "8ba48385ca00273bd8bd2bf7e2c46d291e6e5ad1",
  "build_run": "34830810935",
  "signing_run": "34835395867"
}
```

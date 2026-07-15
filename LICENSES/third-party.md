# Third-Party Licenses

Companion SDKs are build inputs, not redistributed library payloads.

| Component | Locked revision | License policy |
| --- | --- | --- |
| CLAP 1.1.9 | `094bb76c85366a13cc6c49292226d8608d6ae50c` | MIT |
| Steinberg VST3 SDK 3.7.11 | `7d92338ae922db2d559ac458824a4df40f37e82e` | GPL-3.0-only or integrator-held `LicenseRef-Steinberg-VST3` |
| Apple AudioUnitSDK 1.0.0 | `53ea94e5efebf864b70afb673bdd60c977818ec7` | Apache-2.0 |

Exact repositories, submodules, trees, and license URLs are recorded in
`ci/reference-sdks.lock.toml`. Registry dependency license files are retained
inside the deterministic source bundle's `vendor/` directory.

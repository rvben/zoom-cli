# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).



## [0.2.2](https://github.com/rvben/zoom-cli/compare/v0.2.1...v0.2.2) - 2026-04-03

## [0.2.1](https://github.com/rvben/zoom-cli/compare/v0.2.0...v0.2.1) - 2026-04-02

### Added

- show pagination progress when fetching multiple pages ([e5e3364](https://github.com/rvben/zoom-cli/commit/e5e336472b9cb153afed1d5020d0a758b2342359))
- **config**: add zoom config delete command ([5841781](https://github.com/rvben/zoom-cli/commit/5841781a42e68c0c3568e9f16b06030fa19ec9ed))
- **reports**: add zoom reports participants command ([fcf08dc](https://github.com/rvben/zoom-cli/commit/fcf08dcf19186a3613f84af701d67114abd27ce6))
- **recordings**: add transcript download command ([c7b8fff](https://github.com/rvben/zoom-cli/commit/c7b8fff658e57b296d27e2632cf98e9187925ef8))
- **users**: add create, deactivate, and activate commands ([eb4fc29](https://github.com/rvben/zoom-cli/commit/eb4fc2900837add72d7ecc75227c6e8ae68a5517))
- **meetings**: add zoom meetings invite command ([051e7e6](https://github.com/rvben/zoom-cli/commit/051e7e6e4471ae01d30e3102c78ca18fe602fe17))

### Fixed

- add clickable link to missing-scope error message ([c71f3ba](https://github.com/rvben/zoom-cli/commit/c71f3bae31479e7fd3103dff05800a6b2afd1e4f))
- parse Zoom error JSON and give actionable guidance for scope errors ([9ffac38](https://github.com/rvben/zoom-cli/commit/9ffac3889c124684a354be1f05b3538a1837b7fe))
- hoist safe_topic out of per-file loop in recordings::download ([fa409c8](https://github.com/rvben/zoom-cli/commit/fa409c84f5363c1261a7c2f97b60ccb0873d3861))
- **config,recordings**: exit non-zero when config delete aborts non-interactively, output structured JSON from transcript ([ed6f9f7](https://github.com/rvben/zoom-cli/commit/ed6f9f73d1379d64394e1747d2c77fb04c3d6c1c))

## [0.2.0](https://github.com/rvben/zoom-cli/compare/v0.1.0...v0.2.0) - 2026-04-02

### Added

- **init**: redesign interactive setup with excellent UX ([2bc79df](https://github.com/rvben/zoom-cli/commit/2bc79dfc82e18f476e69ea7aaaa4416d3fce2c23))
- **config**: add zoom config show command ([dbe8490](https://github.com/rvben/zoom-cli/commit/dbe84901f3618e6c17542ae6fd8e5c52a741eb29))

### Fixed

- double-encode recording UUIDs and write downloads atomically ([f346939](https://github.com/rvben/zoom-cli/commit/f346939569336d808e5c02f7c2029c55e7273169))
- unreachable!() in send_with_retry was reachable, causing panics ([ceb905d](https://github.com/rvben/zoom-cli/commit/ceb905d7b12dfbc25fa8a7150daf2a5f09158375))
- transparent token refresh on 401, --permanent flag for recordings delete ([fdae155](https://github.com/rvben/zoom-cli/commit/fdae155e096e953c4a6634a177abb9c7ce2ab6cf))

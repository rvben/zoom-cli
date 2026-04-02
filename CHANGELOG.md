# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [0.2.0](https://github.com/rvben/zoom-cli/compare/v0.1.0...v0.2.0) - 2026-04-02

### Added

- **init**: redesign interactive setup with excellent UX ([2bc79df](https://github.com/rvben/zoom-cli/commit/2bc79dfc82e18f476e69ea7aaaa4416d3fce2c23))
- **config**: add zoom config show command ([dbe8490](https://github.com/rvben/zoom-cli/commit/dbe84901f3618e6c17542ae6fd8e5c52a741eb29))

### Fixed

- double-encode recording UUIDs and write downloads atomically ([f346939](https://github.com/rvben/zoom-cli/commit/f346939569336d808e5c02f7c2029c55e7273169))
- unreachable!() in send_with_retry was reachable, causing panics ([ceb905d](https://github.com/rvben/zoom-cli/commit/ceb905d7b12dfbc25fa8a7150daf2a5f09158375))
- transparent token refresh on 401, --permanent flag for recordings delete ([fdae155](https://github.com/rvben/zoom-cli/commit/fdae155e096e953c4a6634a177abb9c7ce2ab6cf))

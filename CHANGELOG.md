# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-08-31

### Added

- Support SQL/JSONPath 2023 numeric, string, boolean, and datetime item methods.
- Add timezone-aware query APIs for PostgreSQL-compatible datetime comparisons.
- Expand PostgreSQL JSONPath regression coverage, with 905 of 916 cases passing.

### Changed

- Preserve SQL/JSON datetime types during path evaluation and improve PostgreSQL compatibility.

## [0.1.1] - 2025-09-26

- Bump dependencies to support `simd-json 0.16` and `jsonbb 0.2`.

## [0.1.0] - 2023-11-30

- Inital release.

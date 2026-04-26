# Roadmap

## Goal

Reach a secure macOS MVP first, then add Windows without rewriting provider logic.

## Phase 0: Foundation

Target: 2 to 3 days

Deliverables:

- repository structure
- product scope freeze
- provider shortlist
- architecture decision record for:
  - credential store
  - local API default
  - telemetry default
  - updater policy

Exit criteria:

- shared data model agreed
- macOS-first but Windows-ready package structure created

## Phase 1: Core and Provider Contracts

Target: 4 to 6 days

Deliverables:

- shared snapshot types
- provider registry
- refresh orchestration
- cache and redaction utilities
- provider interface and error model

Exit criteria:

- providers can be implemented without touching shell code
- security boundaries are encoded in interfaces

## Phase 2: macOS MVP

Target: 5 to 7 days

Deliverables:

- tray integration
- compact dashboard window
- settings screen
- local settings persistence
- credential store adapter for macOS
- first provider set:
  - Claude
  - Codex
  - Cursor
  - Copilot

Exit criteria:

- user can install app on macOS
- configure supported providers
- refresh and view normalized usage snapshots

## Phase 3: Security Hardening

Target: 3 to 5 days

Deliverables:

- log redaction coverage
- diagnostics boundaries
- local API opt-in design
- telemetry opt-in flow
- updater disabled or signed-only release path

Exit criteria:

- default install keeps telemetry and local API off
- no raw secrets in logs or user-facing diagnostics

## Phase 4: Windows Shell

Target: 7 to 10 days

Deliverables:

- Windows tray integration
- compact dashboard window behavior
- Windows credential store adapter
- startup integration
- packaging baseline

Exit criteria:

- shared providers run on Windows shell
- credential resolution and cache path handling work on Windows

## Phase 5: Cross-Platform Stabilization

Target: 4 to 7 days

Deliverables:

- macOS and Windows smoke test checklist
- provider regression fixtures
- install and update notes
- packaging fixes

Exit criteria:

- both platforms produce testable builds
- core flows behave consistently across OSes

## Initial Backlog

### Product

- define supported provider list for v0
- define feature flags for telemetry, local API, updater
- define compact dashboard information hierarchy

### Engineering

- scaffold monorepo layout
- implement shared snapshot schema
- define credential store interface
- define provider adapter contract
- add redaction utilities

### Platform

- macOS credential adapter
- Windows credential adapter
- tray abstraction
- startup abstraction

### Release

- signing strategy
- package naming
- versioning policy
- diagnostics policy

## Success Definition

Version `0.1.0` should mean:

- macOS works for real users
- Windows has a working internal build
- provider logic is shared
- security defaults are conservative
- no dependency on a macOS-only architecture choice

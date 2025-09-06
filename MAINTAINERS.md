# MAINTAINERS

This document lists the current maintainers of the `wasmcp` project and describes
their responsibilities and how maintainership evolves over time.

## Current Maintainers

| Name         | GitHub            | Email                       | Affiliation            |
|--------------|-------------------|-----------------------------|------------------------|
| Ian McDonald | @bowlofarugula     | bowlofarugula@gmail.com     | [fastertools](https://github.com/fastertools) |
| Corey Ryan   | @DevOpsDali  | corey@coreyr.com            | [fastertools](https://github.com/fastertools) |

## Responsibilities

Maintainers:
- Review and merge pull requests.
- Cut releases and publish artifacts.
- Shepherd RFCs and make sure decisions are recorded.
- Triage issues and keep the roadmap healthy.
- Enforce the Code of Conduct and uphold project values.

## Decision Process

Day-to-day decisions happen via PR review. Substantial changes
(e.g., breaking WIT updates, governance edits) follow the RFC process
outlined in `CONTRIBUTING.md` and require maintainer approval
(simple majority if consensus cannot be reached).

## Becoming a Maintainer

We welcome new maintainers. Typical signals include:
- Several non-trivial contributions across code/spec/tests/docs.
- Consistent, constructive review participation.
- Demonstrated alignment with project values (neutrality, openness, interop).

**Process:** An existing maintainer nominates a contributor via an issue or RFC.
After public discussion, a simple majority of current maintainers approves.

## Inactivity & Emeritus

If a maintainer is inactive (no reviews or contributions) for ~6 months,
remaining maintainers may move them to **Emeritus** status
(reversible by the same process as becoming a maintainer).

### Emeritus Maintainers

_None yet._

## Release Managers

Maintainers rotate release duties. The maintainer who opens the release PR is
the “Release Manager” for that version and handles:
- Changelog accuracy
- Tagging and artifact signing
- Publishing release notes (including conformance/interop status)

## Security Contact

For security issues, please follow `SECURITY.md`. Primary contact:
**security@wasmcp.org** (escrowed to current maintainers).

## Communication

- Issues/PRs on GitHub are the primary venue.  
- Ad-hoc community syncs may be scheduled; notes will be published in-repo.

# GitHub Workflows

This directory contains Remnant-maintained GitHub Actions workflows.

## Workflow dependency policy

Third-party GitHub Actions used by Remnant-maintained workflows should be pinned to full commit SHAs instead of mutable tags. Include a trailing comment indicating the version tag the pinned commit corresponds to, such as `# v4.3.1`, so reviewers can understand the pin without an external lookup.

When updating a pinned action SHA:

1. Verify the new commit belongs to the expected upstream repository.
2. Review the upstream release notes or relevant diff before accepting the change.
3. Run the normal Remnant validation checks.
4. Run the Remnant inspect example workflow against its deterministic npm `.tgz` fixture.

## Remnant inspect example

The `remnant-inspect-example.yml` workflow generates an inert npm package artifact and runs the local Remnant inspect action against that artifact.

This validates Remnant's npm artifact inspection path and GitHub Action wrapper behavior. It does not inspect or admit third-party GitHub Action source repositories.

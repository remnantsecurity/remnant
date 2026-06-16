# GitHub Actions

This directory contains Remnant-maintained local composite actions.

These actions wrap Remnant CLI behavior for CI workflows. They should not introduce network ingestion, package execution, telemetry, hidden admission logic, or package-controlled code execution.

Action changes should preserve Remnant's deterministic inspection model and should be validated through the relevant workflows.

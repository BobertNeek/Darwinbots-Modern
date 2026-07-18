#!/usr/bin/env python3
import json
import re
import sys
from collections import Counter
from pathlib import Path


audit_dir = Path(__file__).resolve().parent
repo_root = audit_dir.parents[2]
matrix_path = repo_root / "docs" / "parity" / "control-surface-matrix.json"
test_path = (
    repo_root
    / "modern"
    / "desktop"
    / "tests"
    / "Darwinbots.Desktop.Tests"
    / "RenderedControlSurfaceAuditTests.cs"
)

rows = json.loads(matrix_path.read_text(encoding="utf-8-sig"))
required = {
    "id",
    "surface",
    "label",
    "control_type",
    "visibility",
    "enabled_conditions",
    "modern_handler_or_binding",
    "state_affected",
    "original_db2_equivalent",
    "original_default",
    "modern_default",
    "original_range_or_options",
    "modern_range_or_options",
    "persistence",
    "live_update",
    "interactive_test_id",
    "original_screenshot",
    "modern_screenshot",
    "result",
    "defect_severity",
    "notes",
    "status",
}
allowed_statuses = {
    "confirmed-parity",
    "source-parity",
    "intentional-difference",
    "provisional-wine-result",
    "broken",
    "missing",
    "blocked",
}
expected_surfaces = {
    "SetupWindow": 32,
    "MainWindow": 39,
    "AdvancedSettingsWindow": 34,
    "DnaEditorWindow": 4,
}
errors = []

if len(rows) != 109:
    errors.append(f"expected 109 controls, found {len(rows)}")

ids = [row.get("id") for row in rows]
if len(set(ids)) != len(ids):
    errors.append("matrix contains duplicate control IDs")

for index, row in enumerate(rows):
    missing = sorted(required - row.keys())
    if missing:
        errors.append(f"row {index} missing: {', '.join(missing)}")
    if row.get("status") not in allowed_statuses:
        errors.append(f"row {index} has invalid status: {row.get('status')}")
    if row.get("result") != "pass":
        errors.append(f"row {index} is not passing: {row.get('id')}")
    if row.get("interactive_test_id") != row.get("id"):
        errors.append(f"row {index} test ID differs from control ID: {row.get('id')}")

surface_counts = Counter(row.get("surface") for row in rows)
if dict(surface_counts) != expected_surfaces:
    errors.append(f"unexpected surface counts: {dict(surface_counts)}")

test_text = test_path.read_text(encoding="utf-8-sig")
test_ids = set(re.findall(r'"((?:setup|live|advanced|dna)\.[^"]+)"', test_text))
matrix_ids = set(ids)
if test_ids != matrix_ids:
    errors.append(f"missing from matrix: {sorted(test_ids - matrix_ids)}")
    errors.append(f"missing from rendered test: {sorted(matrix_ids - test_ids)}")

if errors:
    print("\n".join(errors))
    sys.exit(1)

print(f"validated {len(rows)} passing control-surface rows against rendered test coverage")

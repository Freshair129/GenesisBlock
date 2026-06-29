#!/usr/bin/env python3
"""Tests for the Independent Benchmark result verifier and schema.

Stdlib unittest only — no pip dependencies. Run from the repo root:

    python -m unittest benchmark.test_verify_report -v
    # or
    python benchmark/test_verify_report.py
"""
from __future__ import annotations

import copy
import json
import os
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
FIX = os.path.join(HERE, "fixtures")

import importlib.util


def _load(name: str):
    spec = importlib.util.spec_from_file_location(name, os.path.join(HERE, f"{name}.py"))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


verify_report = _load("verify_report")


def load_fixture(name: str) -> dict:
    with open(os.path.join(FIX, name), "r", encoding="utf-8") as f:
        return json.load(f)


class SchemaValidationTests(unittest.TestCase):
    def setUp(self):
        self.schema = verify_report.load_schema()

    def test_passing_sample_matches_schema(self):
        errors: list[str] = []
        verify_report.validate_schema(load_fixture("soak_heavy_12h_pass.json"), self.schema, "$", errors)
        self.assertEqual(errors, [], f"unexpected schema errors: {errors}")

    def test_schema_flags_missing_required_field(self):
        report = load_fixture("soak_heavy_12h_pass.json")
        del report["environment"]
        errors: list[str] = []
        verify_report.validate_schema(report, self.schema, "$", errors)
        self.assertTrue(any("environment" in e for e in errors), errors)

    def test_schema_flags_wrong_type(self):
        report = load_fixture("soak_heavy_12h_pass.json")
        report["results"]["total_nodes"] = "lots"  # should be integer
        errors: list[str] = []
        verify_report.validate_schema(report, self.schema, "$", errors)
        self.assertTrue(any("total_nodes" in e for e in errors), errors)


class VerifierTests(unittest.TestCase):
    def test_accepts_complete_passing_12h(self):
        errors = verify_report.verify(load_fixture("soak_heavy_12h_pass.json"))
        self.assertEqual(errors, [], f"expected PASS, got: {errors}")

    def test_accepts_complete_passing_smoke(self):
        errors = verify_report.verify(load_fixture("soak_smoke_pass.json"))
        self.assertEqual(errors, [], f"expected PASS, got: {errors}")

    def test_accepts_graph_descriptive_benchmark(self):
        errors = verify_report.verify(load_fixture("graph_traversal_pass.json"))
        self.assertEqual(errors, [], f"expected PASS, got: {errors}")

    def test_rejects_missing_commit(self):
        errors = verify_report.verify(load_fixture("bad_missing_commit.json"))
        self.assertTrue(any("commit" in e for e in errors), errors)

    def test_rejects_incomplete_12h(self):
        errors = verify_report.verify(load_fixture("bad_incomplete_12h.json"))
        self.assertTrue(any("12h" in e for e in errors), errors)

    def test_rejects_missing_reopen(self):
        errors = verify_report.verify(load_fixture("bad_missing_reopen.json"))
        self.assertTrue(any("reopen" in e for e in errors), errors)

    def test_rejects_dirty_repo_by_default(self):
        errors = verify_report.verify(load_fixture("bad_dirty_repo.json"))
        self.assertTrue(any("dirty" in e for e in errors), errors)

    def test_allows_dirty_repo_when_flagged(self):
        errors = verify_report.verify(load_fixture("bad_dirty_repo.json"), allow_dirty=True)
        self.assertEqual(errors, [], f"expected PASS with --allow-dirty, got: {errors}")

    def test_rejects_failed_run(self):
        report = load_fixture("soak_heavy_12h_pass.json")
        report["results"]["pass"] = False
        errors = verify_report.verify(report)
        self.assertTrue(any("pass" in e for e in errors), errors)

    def test_rejects_interrupted_run(self):
        report = load_fixture("soak_heavy_12h_pass.json")
        report["interrupted"] = True
        errors = verify_report.verify(report)
        self.assertTrue(any("interrupted" in e for e in errors), errors)

    def test_rejects_missing_latency(self):
        report = load_fixture("soak_heavy_12h_pass.json")
        report["results"]["query_latency_p95_ms"] = None
        errors = verify_report.verify(report)
        self.assertTrue(any("query_latency_p95_ms" in e for e in errors), errors)

    def test_rejects_zero_total_nodes(self):
        report = load_fixture("soak_smoke_pass.json")
        report["results"]["total_nodes"] = 0
        errors = verify_report.verify(report)
        self.assertTrue(any("total_nodes" in e for e in errors), errors)

    def test_rejects_missing_environment_os(self):
        report = load_fixture("soak_smoke_pass.json")
        report["environment"]["os"] = None
        errors = verify_report.verify(report)
        self.assertTrue(any("os" in e for e in errors), errors)


if __name__ == "__main__":
    unittest.main(verbosity=2)

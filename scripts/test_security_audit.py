"""Verify the read-only audit gate preserves cargo-audit failures."""

import unittest
from unittest.mock import patch

import run_security_audit


class SecurityAuditTests(unittest.TestCase):
    def test_warning_only_success(self):
        with patch.object(run_security_audit.subprocess, "call", return_value=0):
            self.assertEqual(run_security_audit.main(), 0)

    def test_vulnerability_failure_is_preserved(self):
        with patch.object(run_security_audit.subprocess, "call", return_value=1):
            self.assertEqual(run_security_audit.main(), 1)

    def test_tool_failure_is_preserved(self):
        with patch.object(run_security_audit.subprocess, "call", return_value=2):
            self.assertEqual(run_security_audit.main(), 2)

    def test_only_registered_exceptions_are_passed(self):
        with patch.object(run_security_audit.subprocess, "call", return_value=0) as call:
            run_security_audit.main()
        command = call.call_args.args[0]
        self.assertEqual(command[:4], ["cargo", "audit", "--file", "Cargo.lock"])
        self.assertEqual(command[4:], [
            "--ignore", "RUSTSEC-2025-0141",
            "--ignore", "RUSTSEC-2026-0204",
        ])


if __name__ == "__main__":
    unittest.main()

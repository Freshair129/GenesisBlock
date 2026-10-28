"""Verify the read-only audit gate preserves cargo-audit failures."""

import unittest
from unittest.mock import patch

import run_security_audit


class SecurityAuditTests(unittest.TestCase):
    def test_warning_only_success(self):
        with patch.object(
            run_security_audit.subprocess, "call", side_effect=[0, 0, 0]
        ) as call:
            self.assertEqual(run_security_audit.main(), 0)
        self.assertEqual(call.call_count, 3)

    def test_vulnerability_failure_is_preserved(self):
        with patch.object(
            run_security_audit.subprocess, "call", side_effect=[1, 0, 0]
        ) as call:
            self.assertEqual(run_security_audit.main(), 1)
        self.assertEqual(call.call_count, 3)

    def test_tool_failure_is_preserved(self):
        with patch.object(
            run_security_audit.subprocess, "call", side_effect=[0, 2, 0]
        ) as call:
            self.assertEqual(run_security_audit.main(), 2)
        self.assertEqual(call.call_count, 3)

    def test_every_lockfile_uses_only_registered_exceptions(self):
        with patch.object(
            run_security_audit.subprocess, "call", side_effect=[0, 0, 0]
        ) as call:
            run_security_audit.main()
        expected_lockfiles = [
            "Cargo.lock",
            "studio/src-tauri/Cargo.lock",
            "genesisblock-mobile/src-tauri/Cargo.lock",
        ]
        expected_exceptions = [
            "--ignore", "RUSTSEC-2025-0141",
            "--ignore", "RUSTSEC-2026-0204",
        ]
        commands = [entry.args[0] for entry in call.call_args_list]
        self.assertEqual(len(commands), len(expected_lockfiles))
        for command, lockfile in zip(commands, expected_lockfiles):
            self.assertEqual(command[:4], ["cargo", "audit", "--file", lockfile])
            self.assertEqual(command[4:], expected_exceptions)


if __name__ == "__main__":
    unittest.main()

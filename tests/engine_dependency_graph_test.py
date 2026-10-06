"""Regression coverage for native dependency isolation checks."""
import subprocess
import unittest
from unittest.mock import patch

import engine_dependency_graph as guard


class DependencyGuard(unittest.TestCase):
    def test_transport_families_and_duplicate_entries_are_rejected(self):
        graph = "\n".join([
            "hyperlocal v0.9.1", "hyperlocal v0.9.1 (*)", "hyper-util v0.1.20",
            "axum-core v0.5.0", "tokio-util v0.7.0", "reqwest v0.12.0",
            "sqlx-core v0.8.0", "rustls-pki-types v1.0.0", "multer v3.0.0",
        ])
        self.assertEqual(guard.native_dependencies(graph), [
            "axum-core", "hyper-util", "hyperlocal", "multer", "reqwest",
            "rustls-pki-types", "sqlx-core", "tokio-util",
        ])

    def test_oidc_implementation_is_rejected(self):
        self.assertEqual(guard.native_dependencies("openidconnect v4.0.1\noauth2 v5.0.0\nrsa v0.9.10"),
                         ["oauth2", "openidconnect", "rsa"])

    def test_host_http_types_and_package_paths_are_allowed(self):
        graph = "\n\nhttp v1.0.0\nhttp-body v1.0.0\nhttp-body-util v0.1.0\nbytes v1.0.0\n"
        graph += "my-consumer v0.1.0 (/tmp/hyperlocal)\nhyperlocality v0.1.0\n"
        self.assertEqual(guard.native_dependencies(graph), [])

    @patch.object(guard.subprocess, "check_output", return_value="hyperlocal v0.9.1\n")
    def test_leak_fails_the_check_with_fixture_and_package_diagnostics(self, cargo):
        with patch("sys.stderr") as stderr:
            self.assertEqual(guard.main(), 1)
        message = "".join(call.args[0] for call in stderr.write.call_args_list)
        self.assertIn("engine_consumer", message)
        self.assertIn("hyperlocal", message)
        self.assertEqual(cargo.call_args.args[0][-4:], ["--edges", "normal,build", "--prefix", "none"])

    @patch.object(guard.subprocess, "check_output", return_value="http v1.0.0\n")
    def test_every_fixture_is_checked(self, cargo):
        with patch("sys.stdout"):
            self.assertEqual(guard.main(), 0)
        self.assertEqual(cargo.call_count, len(guard.FIXTURES))
        for call, fixture in zip(cargo.call_args_list, guard.FIXTURES):
            self.assertIn(str(guard.ROOT / "tests/fixtures" / fixture / "Cargo.toml"), call.args[0])

    @patch.object(guard.subprocess, "check_output", side_effect=subprocess.CalledProcessError(1, "cargo"))
    def test_cargo_failure_does_not_pass_as_an_empty_graph(self, _cargo):
        with self.assertRaises(subprocess.CalledProcessError):
            guard.main()


if __name__ == "__main__":
    unittest.main()

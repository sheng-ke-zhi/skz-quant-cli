"""Local TLS regression; requires Python 3 and openssl, writes only temporary files."""

import http.server
import json
import os
from pathlib import Path
import ssl
import subprocess
import sys
import tempfile
import threading


def openssl(directory, *args):
    subprocess.run(
        ["openssl", *args], cwd=directory, check=True, capture_output=True
    )


class Handler(http.server.BaseHTTPRequestHandler):
    oversized_calls = 0

    def do_GET(self):
        if self.path.startswith("/research/portfolios/too-large"):
            Handler.oversized_calls += 1
            self.send_response(200)
            self.send_header("Content-Length", str(64 * 1024 * 1024 + 1))
            self.end_headers()
            try:
                for _ in range(64):
                    self.wfile.write(b"x" * (1024 * 1024))
                self.wfile.write(b"x")
            except OSError:
                pass  # The client may close immediately after detecting the limit.
        elif self.path == "/research/whoami":
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b'{"code":0,"data":{"user_id":"tls-test"}}')
        else:
            # Reaching this HTTP response proves OAuth discovery passed TLS.
            self.send_response(403)
            self.end_headers()

    do_POST = do_GET

    def log_message(self, *_args):
        pass


with tempfile.TemporaryDirectory() as directory:
    root = Path(directory)
    for name in ("trusted", "untrusted"):
        openssl(root, "req", "-x509", "-newkey", "rsa:2048", "-nodes",
                "-keyout", f"{name}.key", "-out", f"{name}.pem", "-days", "1",
                "-subj", f"/CN={name}", "-addext", "basicConstraints=critical,CA:TRUE")
    openssl(root, "req", "-newkey", "rsa:2048", "-nodes", "-keyout", "leaf.key",
            "-out", "leaf.csr", "-subj", "/CN=localhost")
    (root / "extensions").write_text(
        "subjectAltName=DNS:localhost\nbasicConstraints=critical,CA:FALSE\n"
        "keyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\n"
    )
    openssl(root, "x509", "-req", "-in", "leaf.csr", "-CA", "trusted.pem",
            "-CAkey", "trusted.key", "-CAcreateserial", "-out", "leaf.pem",
            "-days", "1", "-extfile", "extensions")
    credentials = root / "skz" / "credentials"
    credentials.parent.mkdir()
    credentials.write_text("local-test-token")
    (root / "empty-certs").mkdir()
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.load_cert_chain(root / "leaf.pem", root / "leaf.key")
    server.socket = context.wrap_socket(server.socket, server_side=True)
    threading.Thread(target=server.serve_forever, daemon=True).start()

    def run(ca, host, args):
        env = os.environ.copy()
        for key in list(env):
            if key.lower().endswith("_proxy") or key == "SKZ_READ_ONLY":
                env.pop(key)
        env.update(
            HOME=str(root), XDG_CONFIG_HOME=str(root),
            SSL_CERT_FILE=str(root / f"{ca}.pem"),
            SSL_CERT_DIR=str(root / "empty-certs"),
            SKZ_BASE_URL=f"https://{host}:{server.server_port}",
        )
        return subprocess.run([sys.argv[1], *args], env=env, capture_output=True, timeout=15)

    try:
        result = run("trusted", "localhost", ["whoami"])
        assert result.returncode == 0, result.stderr
        assert json.loads(result.stdout)["user_id"] == "tls-test"
        for ca, host in [("untrusted", "localhost"), ("trusted", "127.0.0.1")]:
            result = run(ca, host, ["whoami"])
            assert result.returncode != 0 and not result.stdout, result
            result = run(ca, host, ["auth", "login", "tls", "--read-only", "--no-browser"])
            assert json.loads(result.stderr)["error"]["kind"] == "network", result.stderr
        result = run("trusted", "localhost", ["auth", "login", "tls", "--read-only", "--no-browser"])
        assert "服务器未提供 OAuth 配置发现接口" in result.stderr.decode(), result.stderr
        for operation, expected_code, expected_action in [
            ("get", 6, "internal"), ("refresh", 7, "check_existing")
        ]:
            result = run("trusted", "localhost", ["portfolio", operation, "too-large"])
            error = json.loads(result.stderr)["error"]
            assert result.returncode == expected_code, result.stderr
            assert error["action"] == expected_action, error
            assert "67108864" in error["message"], error
            assert not result.stdout
            if operation == "refresh":
                assert "refresh-status" in error["remediation"]["verifyWith"], error
        assert Handler.oversized_calls == 2, Handler.oversized_calls
    finally:
        server.shutdown()
        server.server_close()

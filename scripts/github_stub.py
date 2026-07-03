#!/usr/bin/env python3
"""Local stand-in for api.github.com used by the TUI e2e harness.

Resolves any /repos/:owner/:name with a deterministic id, returns an empty
contributors list, one stub open issue from /repos/:owner/:name/issues, and
404s owner "ghost" to exercise the unresolvable-repo path. Mirrors the
fixture server in fido-server/tests/e2e_community_rewrite.rs.
"""

import json
import re
import sys
from http.server import BaseHTTPRequestHandler, HTTPServer

REPO_RE = re.compile(r"^/repos/([^/]+)/([^/]+)$")
CONTRIBUTORS_RE = re.compile(r"^/repos/([^/]+)/([^/]+)/contributors$")
ISSUES_RE = re.compile(r"^/repos/([^/]+)/([^/]+)/issues$")


def repo_id(owner: str, name: str) -> int:
    h = 7
    for b in f"{owner}/{name}".encode():
        h = (h * 31 + b) % (2**63)
    return max(h, 2)


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        path = self.path.split("?")[0]
        if CONTRIBUTORS_RE.fullmatch(path):
            return self._reply(200, [])

        if ISSUES_RE.fullmatch(path):
            return self._reply(
                200,
                [
                    {
                        "id": 1,
                        "number": 7,
                        "title": "Stub issue for e2e",
                        "state": "open",
                        "html_url": "https://github.com/stub/repo/issues/7",
                        "created_at": "2026-07-03T00:00:00Z",
                        "user": {"login": "stubuser"},
                    }
                ],
            )

        repo = REPO_RE.fullmatch(path)
        if repo:
            owner, name = repo.groups()
            if owner == "ghost":
                return self._reply(404, {"message": "Not Found"})
            return self._reply(
                200,
                {
                    "id": repo_id(owner, name),
                    "name": name,
                    "full_name": f"{owner}/{name}",
                    "private": False,
                    "owner": {"login": owner},
                },
            )

        return self._reply(404, {"message": "Not Found"})

    def _reply(self, code, obj):
        body = json.dumps(obj).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, format, *args):
        sys.stderr.write("stub: %s\n" % (format % args))


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: github_stub.py <port>")
    port = int(sys.argv[1])
    HTTPServer(("127.0.0.1", port), Handler).serve_forever()


if __name__ == "__main__":
    main()

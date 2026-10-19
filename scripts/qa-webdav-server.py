"""Independent WsgiDAV integration fixture. Loopback and disposable test data only.
Not part of the application, not an authenticated user's server.
pip install wsgidav==4.3.3 cheroot==11.1.2 in an isolated environment first.
"""
import json
import tempfile
import sys
import threading
import hashlib
from pathlib import Path
from cheroot import wsgi
from wsgidav.wsgidav_app import WsgiDAVApp

workspace = Path(__file__).resolve().parent.parent
parent = workspace / "artifacts" / "qa-sync"
parent.mkdir(parents=True, exist_ok=True)
root = Path(tempfile.mkdtemp(prefix="dav-", dir=parent))
(root / "TEST-DATA-ONLY.txt").write_text("Disposable ResearchClip integration fixture", encoding="utf-8")
app = WsgiDAVApp({"provider_mapping": {"/": str(root)}, "simple_dc": {"user_mapping": {"*": True}}, "http_authenticator": {"accept_basic": True, "accept_digest": False, "default_to_digest": False}, "verbose": 0, "logging": {"enable": False}})
strict = "--serialize-writes" in sys.argv
tls = "--tls" in sys.argv
port = 1462 if tls else 1461 if strict else 1459
if strict:
    # WsgiDAV's Windows default uses integer-second mtime and size: equal-size
    # edits within one second can retain an ETag. This distinct fixture uses
    # content SHA256, in addition to serializing check+write. It is NOT evidence
    # that stock WsgiDAV or a cloud provider has either guarantee.
    from wsgidav import util
    def content_etag(path):
        digest = hashlib.sha256()
        with open(path, "rb") as source:
            for chunk in iter(lambda: source.read(65536), b""):
                digest.update(chunk)
        return digest.hexdigest()
    util.get_file_etag = content_etag
write_lock = threading.Lock()
def serialized(environ, start_response):
    # A separate server configuration with explicit conditional-check/write
    # serialization. Raw WsgiDAV failures are retained, never called PASS.
    with write_lock:
        yield from app(environ, start_response)
server = wsgi.Server(("127.0.0.1", port), serialized if strict else app)
if tls:
    from cheroot.ssl.builtin import BuiltinSSLAdapter
    certs = parent / "certs-v300"
    server.ssl_adapter = BuiltinSSLAdapter(str(certs / "cert.pem"), str(certs / "key.pem"))
(parent / f"server-{port}.json").write_text(json.dumps({"mode": "isolated WsgiDAV4.3.3; HTTP loopback; not Nutstore compatibility", "root": str(root), "url": f"{'https' if tls else 'http'}://127.0.0.1:{port}/", "strict": strict}), encoding="utf-8")
print(f"Independent WsgiDAV fixture on 127.0.0.1:{port}; serialized={strict}; disposable root created", flush=True)
server.start()

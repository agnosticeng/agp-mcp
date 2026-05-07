import json
import subprocess
import time
import urllib.request
import urllib.error
import threading
import sys
import os
from http.server import HTTPServer, BaseHTTPRequestHandler

# --- Mock ClickHouse Proxy ---
class MockClickHouseHandler(BaseHTTPRequestHandler):
    def do_POST(self):
        content_length = int(self.headers.get('Content-Length', 0))
        post_data = self.rfile.read(content_length).decode('utf-8')
        
        if "system.tables" in post_data:
            response = {
                "meta": [{"name": "name", "type": "String"}, {"name": "create_table_query", "type": "String"}],
                "data": [{"name": "users", "create_table_query": "CREATE TABLE users (id Int32) ENGINE = MergeTree ORDER BY id"}],
                "rows": 1,
                "statistics": {"bytes_read": 100, "elapsed": 0.1, "rows_read": 1}
            }
        else:
            response = {
                "meta": [{"name": "result", "type": "Int32"}],
                "data": [{"result": 42}],
                "rows": 1,
                "statistics": {"bytes_read": 10, "elapsed": 0.01, "rows_read": 1}
            }
            
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.end_headers()
        self.wfile.write(json.dumps(response).encode('utf-8'))

    def log_message(self, format, *args):
        return

def run_mock_clickhouse(port):
    server = HTTPServer(('127.0.0.1', port), MockClickHouseHandler)
    server.serve_forever()

# --- Helper Functions ---
def send_mcp_request(process, request):
    req_json = json.dumps(request) + "\n"
    process.stdin.write(req_json.encode('utf-8'))
    process.stdin.flush()
    
    # Read response (might be multiple lines if there are logs, so we look for JSON)
    while True:
        line = process.stdout.readline().decode('utf-8')
        if not line:
            return None
        try:
            return json.loads(line)
        except json.JSONDecodeError:
            continue

# --- Test Suites ---
def test_stdio(binary_path, proxy_url):
    print(">>> Testing STDIO transport")
    process = subprocess.Popen(
        [binary_path, "--url", proxy_url],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE
    )

    try:
        # 1. Initialize
        print("Initializing...")
        resp = send_mcp_request(process, {
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1"}
            }
        })
        assert resp['id'] == 1
        assert resp['result']['serverInfo']['name'] == 'agp-mcp'

        # 2. List tools
        print("Listing tools...")
        resp = send_mcp_request(process, {
            "jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}
        })
        assert 'get_schema' in [t['name'] for t in resp['result']['tools']]

        # 3. Call get_schema
        print("Calling get_schema...")
        resp = send_mcp_request(process, {
            "jsonrpc": "2.0", "id": 3, "method": "tools/call",
            "params": {"name": "get_schema", "arguments": {}}
        })
        assert 'users' in resp['result']['content'][0]['text']
        print("STDIO transport PASSED")
    finally:
        process.terminate()
        process.wait()

def test_http(binary_path, proxy_url, mcp_port):
    print(f"\n>>> Testing HTTP transport on port {mcp_port}")
    env = os.environ.copy()
    env["HTTP_BIND_ADDRESS"] = f"127.0.0.1:{mcp_port}"
    process = subprocess.Popen(
        [binary_path, "--url", proxy_url, "--http"],
        env=env,
        stderr=subprocess.PIPE
    )

    mcp_url = f"http://127.0.0.1:{mcp_port}/mcp/"
    
    # Wait for server
    time.sleep(2)

    try:
        # In rmcp, HTTP POST to the main endpoint initiates a session and returns SSE
        print("Initializing session...")
        init_req = {
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1"}
            }
        }
        req = urllib.request.Request(
            mcp_url, 
            data=json.dumps(init_req).encode('utf-8'),
            headers={
                'Content-Type': 'application/json',
                'Accept': 'application/json, text/event-stream'
            }
        )
        
        with urllib.request.urlopen(req) as f:
            sid = f.headers.get('mcp-session-id')
            body = f.read().decode('utf-8')
            assert sid is not None
            assert 'agp-mcp' in body
        
        print(f"Session established: {sid}")

        # Now send a message
        print("Sending message...")
        msg_req = {
            "jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}
        }
        req = urllib.request.Request(
            f"http://127.0.0.1:{mcp_port}/mcp/message",
            data=json.dumps(msg_req).encode('utf-8'),
            headers={
                'Content-Type': 'application/json',
                'Accept': 'application/json, text/event-stream',
                'Mcp-Session-Id': sid
            }
        )
        with urllib.request.urlopen(req) as f:
            # The message response might be delivered via the SSE stream in a real scenario,
            # but usually the POST returns 202 or 200.
            assert f.status in [200, 202]
        
        print("HTTP transport (basic handshake) PASSED")
    finally:
        process.terminate()
        process.wait()

if __name__ == "__main__":
    binary = "./target/debug/agp-mcp"
    if not os.path.exists(binary):
        print("Building binary...")
        subprocess.run(["cargo", "build"], check=True)

    proxy_port = 8125
    mcp_port = 8003
    
    threading.Thread(target=run_mock_clickhouse, args=(proxy_port,), daemon=True).start()
    time.sleep(1)
    
    try:
        test_stdio(binary, f"http://127.0.0.1:{proxy_port}")
        test_http(binary, f"http://127.0.0.1:{proxy_port}", mcp_port)
        print("\nALL E2E TESTS PASSED")
    except Exception as e:
        print(f"\nE2E TEST FAILED: {e}")
        sys.exit(1)

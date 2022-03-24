import socket
import time
import json
import sys

sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
sock.connect("server.sock")
file = sock.makefile(mode='w')
while True:
    ready = {
        "type": "Ready",
    }
    file.write(json.dumps(ready))
    file.write('\0')
    file.flush()
    time.sleep(1)
    # sock.send(bytes(json.dumps(ready), encoding="utf-8"))
    # sock.flush()
    # data = sock.recv(100)
    # if len(data) > 0:
    #     print(data)

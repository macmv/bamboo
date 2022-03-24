import socket
import event
import sys

class Server:
    def __init__(self):
        sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        sock.connect("server.sock")
        self.file = sock.makefile(mode='rw')
        self.buffer = bytearray()

    def send(self, event):
        self.file.write(event.to_json())
        self.file.write('\0')
        self.file.flush()

    def recv(self):
        while True:
            event = self.read_event()
            if event != None:
                return event
            b = bytes(self.file.read(1), encoding="utf-8")
            if len(b) == 0:
                print("connection has been closed, exiting")
                sys.exit(0)
            self.buffer += b

    def read_event(self):
        idx = self.buffer.find(b'\0')
        sys.stdout.flush()
        if idx == -1:
            return None
        data = self.buffer[:idx]
        self.buffer = self.buffer[idx+1:]
        return event.read(data)

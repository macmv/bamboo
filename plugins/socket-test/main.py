import event
import server

server = server.Server()

server.send(event.Ready())

while True:
    ev = server.recv()
    print(ev)
    print(ev.pos)
    server.send(event.SendChat("Hello world!"))
    # sock.send(bytes(json.dumps(ready), encoding="utf-8"))
    # sock.flush()
    # data = sock.recv(100)
    # if len(data) > 0:
    #     print(data)

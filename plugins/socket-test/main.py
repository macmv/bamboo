import event
import server

server = server.Server()

server.send(event.Ready())

while True:
    event = server.recv()
    print(event)
    print(event.pos)
    # sock.send(bytes(json.dumps(ready), encoding="utf-8"))
    # sock.flush()
    # data = sock.recv(100)
    # if len(data) > 0:
    #     print(data)

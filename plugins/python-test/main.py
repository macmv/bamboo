import bamboo

print("Gaming")

def init():
    print("Hello from python!")

def on_tick(event):
    bb = bamboo.instance()
    bb.broadcast(5)

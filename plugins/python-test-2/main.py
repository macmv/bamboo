import bamboo

print("Gaming")

def init():
    print("Hello from python 2!")

def on_tick(event):
    bb = bamboo.instance()
    bb.broadcast(10)

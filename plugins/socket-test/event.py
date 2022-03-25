import json

def read(b):
    blob = json.loads(b)
    match blob["t"]:
        case "BlockPlace": return BlockPlace.from_json(blob)
        case other: print("unknown event " + other)
    return None

class SendChat:
    def __init__(self, text):
        self.text = text

    def to_json(self):
        return json.dumps({
            "t": "SendChat",
            "text": self.text,
        })

class BlockPlace:
    def __init__(self, pos):
        self.pos = pos

    def from_json(blob):
        return BlockPlace(blob["pos"])

class Ready:
    def to_json(self):
        return json.dumps({
            "t": "Ready",
        })

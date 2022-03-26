import json

def read(b):
    blob = json.loads(b)
    match blob["kind"]:
        case "Event":
            match blob["type"]:
                case "BlockPlace": return BlockPlace.from_json(blob)
                case other: print("unknown event " + other)
        case "Reply":
            match blob["type"]:
                case "Block": return Block.from_json(blob)
                case other: print("unknown reply " + other)
    return None

class SendChat:
    def __init__(self, text):
        self.text = text

    def to_json(self):
        return json.dumps({
            "kind": "Event",
            "type": "SendChat",
            "text": self.text,
        })

class Block:
    def __init__(self, pos, block):
        self.pos = pos
        self.block = block

    def from_json(blob):
        return Block(blob["pos"], blob["block"])

class BlockPlace:
    def __init__(self, pos):
        self.pos = pos

    def from_json(blob):
        return BlockPlace(blob["pos"])

class Ready:
    def to_json(self):
        return json.dumps({
            "kind": "Event",
            "type": "Ready",
        })

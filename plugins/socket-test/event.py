import json

def read(b):
    blob = json.loads(b)
    match blob["kind"]:
        case "Event":
            match blob["type"]:
                case "BlockPlace": return BlockPlace.from_json(blob)
                case other: print("unknown event " + other)
        case "Reply":
            reply_id = blob["reply_id"]
            match blob["type"]:
                case "Block": return Block.from_json(reply_id, blob)
                case other: print("unknown reply " + other)
    return None

class Reply:
    def __init__(self, reply_id):
        self.reply_id = reply_id
class Request:
    def __init__(self, reply_id):
        self.reply_id = reply_id

    def to_json(self):
        return {
            "kind": "Request",
            "reply_id": self.reply_id,
        }

class Event:
    pass

class SendChat(Event):
    def __init__(self, text):
        self.text = text

    def to_json(self):
        return {
            "kind": "Event",
            "type": "SendChat",
            "text": self.text,
        }

class GetBlock(Request):
    def __init__(self, reply_id, pos):
        super().__init__(reply_id)
        self.pos = pos

    def to_json(self):
        blob = super().to_json()
        blob["type"] = "GetBlock"
        blob["pos"] = self.pos
        return blob

class Block(Reply):
    def __init__(self, reply_id, pos, block):
        super().__init__(reply_id)
        self.pos = pos
        self.block = block

    def from_json(reply_id, blob):
        return Block(reply_id, blob["pos"], blob["block"])

class BlockPlace(Event):
    def __init__(self, pos):
        self.pos = pos

    def from_json(blob):
        return BlockPlace(blob["pos"])

class Ready(Event):
    def to_json(self):
        return {
            "kind": "Event",
            "type": "Ready",
        }

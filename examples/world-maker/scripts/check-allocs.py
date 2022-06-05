with open("log.txt", "r") as f:
  log = f.read()

allocs = {}
for line in log.splitlines():
  # sample line:
  # 2022-06-05 03:48:32.540 :0 [INFO] freeing    at 0x00110088: 0x8
  sections = line.split(" ")
  if len(line) == 0:
    continue
  _date = sections[0]
  time = sections[1]
  path = sections[2]
  _log_level = sections[3]
  message = " ".join(sections[4:])
  if message.startswith("allocating "):
    addr = message.split(" ")[2][:-1]
    size = message.split(" ")[3]
    if addr in allocs:
      raise Exception("DOUBLE ALLOC!!")
    allocs[addr] = message
  elif message.startswith("freeing  "):
    addr = message.split(" ")[5][:-1]
    size = message.split(" ")[6]
    if not addr in allocs:
      raise Exception("INVALID FREE!!")
    del allocs[addr]

for addr, message in allocs.items():
  print(f"at {time}: {message}")

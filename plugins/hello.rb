module Hello
  BIG = "gaming"

  def self.init(sc)
    puts "Loaded hello plugin"
    sc.broadcast("big gaming")
    @sc = sc
  end

  def self.on_block_place(pos)
    puts "HELLO I AM ON BLOCK PLACE"
    puts pos.x
    @sc.broadcast("someone just placed a block at #{pos}")
    puts Sugarcane::Block::DIRT
  end
end

# p Hello::public_methods - Module::public_methods
# Hello

module Hello
  BIG = "gaming"

  def self.init(sc)
    puts "Loaded hello plugin"
    sc.broadcast("big gaming")
  end

  def self.on_block_place(player, pos)
    Sugarcane::broadcast("#{player.username} just placed a block at #{pos}")
  end
end

# p Hello::public_methods - Module::public_methods
# Hello

module Hello
  BIG = "gaming"

  def self.init
    puts "Loaded hello plugin"
    Sugarcane::broadcast("big gaming")
    other
  end

  def self.other
    asdasd
  end

  def self.on_block_place(player, pos)
    Sugarcane::broadcast("#{player.username} just placed a block at #{pos}")
  end
end

# p Hello::public_methods - Module::public_methods
# Hello

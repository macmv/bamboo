module Hello
  BIG = "gaming"

  def self.init(sc)
    puts "Loaded hello plugin"
    sc.broadcast("big gaming")
    @sc = sc
  end

  def self.on_block_place(pos, kind)
    Sugarcane::info("Hello World! #{pos}")
    Sugarcane::error("mmmmmmmmm error")
    @sc.broadcast("someone just placed a block at #{pos}")
    if kind == Sugarcane::Block::DIRT
      @sc.broadcast("placed dirt!")
    end
  end
end

# p Hello::public_methods - Module::public_methods
# Hello

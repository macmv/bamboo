module Hello
  BIG = "gaming"

  def self.init(sc)
    puts "Loaded hello plugin"
    sc.broadcast("big gaming")
    @sc = sc
  end

  def self.on_block_place(player, pos, kind)
    @sc.broadcast("#{player.username} just placed a block at #{pos}")
    agdfsag
    player.world.set_block(pos, Sugarcane::Block::STONE)
    if kind == Sugarcane::Block::DIRT
      @sc.broadcast("placed dirt!")
    end
  end
end

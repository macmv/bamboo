module Hello
  BIG = "gaming"

  def self.init(sc)
    Sugarcane::info("Loaded hello plugin")
    asdf
    @sc = sc
  end

  def self.on_block_place(player, pos, kind)
    Sugarcane::info("someone just placed a block at #{pos}")
    # player.world.set_block(pos, Sugarcane::Block::STONE)
    asdf
    Sugarcane::info("big gaming energy")
    if kind == Sugarcane::Block::DIRT
      @sc.broadcast("placed dirt!")
    end
  end
end

struct Base3DNoise {
  lower:         Octave<Perlin>,
  upper:         Octave<Perlin>,
  interpolation: Octave<Perlin>,
  xzScale:       f64,
  yScale:        f64,
  xzMainScale:   f64,
  yMainScale:    f64,
  cellWidth:     i32,
  cellHeight:    i32,
  field_36630:   f64,
}

impl Base3DNoise {
  pub fn new(
    lower: Octave<Perlin>,
    upper: Octave<Perlin>,
    interpolation: Octave<Perlin>,
    config: NoiseSamplingConfig,
    cell_width: i32,
    cell_height: i32,
  ) {
    let xz_scale = 684.412 * config.xz_scale();
    let y_scale = 684.412 * config.y_scale();
    Base3DNoise {
      lower,
      upper,
      interpolationNoise,
      xzScale,
      yScale,
      xzMainScale: xz_scale / config.xz_factor(),
      yMainScale: y_scale / config.y_factor(),
      cell_width,
      cell_height,
      field_36630: lowerInterpolatedNoise.method_40556(y_scale),
    }
  }

  /*
    public InterpolatedNoiseSampler(AbstractRandom random, NoiseSamplingConfig config, int cellWidth, int cellHeight) {
        this(OctavePerlinNoiseSampler.createLegacy(random, IntStream.rangeClosed(-15, 0)), OctavePerlinNoiseSampler.createLegacy(random, IntStream.rangeClosed(-15, 0)), OctavePerlinNoiseSampler.createLegacy(random, IntStream.rangeClosed(-7, 0)), config, cellWidth, cellHeight);
    }

    @Override
    public double sample(DensityFunction.NoisePos pos) {
        int i = Math.floorDiv(pos.blockX(), this.cellWidth);
        int j = Math.floorDiv(pos.blockY(), this.cellHeight);
        int k = Math.floorDiv(pos.blockZ(), this.cellWidth);
        double d = 0.0;
        double e = 0.0;
        double f = 0.0;
        boolean bl = true;
        double g = 1.0;
        for (int l = 0; l < 8; ++l) {
            PerlinNoiseSampler perlinNoiseSampler = this.interpolationNoise.getOctave(l);
            if (perlinNoiseSampler != null) {
                f += perlinNoiseSampler.sample(OctavePerlinNoiseSampler.maintainPrecision((double)i * this.xzMainScale * g), OctavePerlinNoiseSampler.maintainPrecision((double)j * this.yMainScale * g), OctavePerlinNoiseSampler.maintainPrecision((double)k * this.xzMainScale * g), this.yMainScale * g, (double)j * this.yMainScale * g) / g;
            }
            g /= 2.0;
        }
        double h = (f / 10.0 + 1.0) / 2.0;
        boolean bl2 = h >= 1.0;
        boolean bl3 = h <= 0.0;
        g = 1.0;
        for (int m = 0; m < 16; ++m) {
            PerlinNoiseSampler perlinNoiseSampler2;
            double n = OctavePerlinNoiseSampler.maintainPrecision((double)i * this.xzScale * g);
            double o = OctavePerlinNoiseSampler.maintainPrecision((double)j * this.yScale * g);
            double p = OctavePerlinNoiseSampler.maintainPrecision((double)k * this.xzScale * g);
            double q = this.yScale * g;
            if (!bl2 && (perlinNoiseSampler2 = this.lowerInterpolatedNoise.getOctave(m)) != null) {
                d += perlinNoiseSampler2.sample(n, o, p, q, (double)j * q) / g;
            }
            if (!bl3 && (perlinNoiseSampler2 = this.upperInterpolatedNoise.getOctave(m)) != null) {
                e += perlinNoiseSampler2.sample(n, o, p, q, (double)j * q) / g;
            }
            g /= 2.0;
        }
        return MathHelper.clampedLerp(d / 512.0, e / 512.0, h) / 128.0;
    }

    @Override
    public double minValue() {
        return -this.maxValue();
    }

    @Override
    public double maxValue() {
        return this.field_36630;
    }

    @VisibleForTesting
    public void addDebugInfo(StringBuilder info) {
        info.append("BlendedNoise{minLimitNoise=");
        this.lowerInterpolatedNoise.addDebugInfo(info);
        info.append(", maxLimitNoise=");
        this.upperInterpolatedNoise.addDebugInfo(info);
        info.append(", mainNoise=");
        this.interpolationNoise.addDebugInfo(info);
        info.append(String.format(", xzScale=%.3f, yScale=%.3f, xzMainScale=%.3f, yMainScale=%.3f, cellWidth=%d, cellHeight=%d", this.xzScale, this.yScale, this.xzMainScale, this.yMainScale, this.cellWidth, this.cellHeight)).append('}');
    }
  */
}

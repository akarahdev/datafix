use datafix::{
    result::CodecResult,
    serialization::{Codec, CodecAdapters, CodecOps, DefaultCodec, MapCodecBuilder, json::JsonOps},
};

#[derive(Clone, Debug, PartialEq)]
pub struct GameConfig {
    volume: i32,
    gamma: i32,
    render_distance: u8,
}

impl GameConfig {
    pub fn volume(&self) -> &i32 {
        &self.volume
    }

    pub fn gamma(&self) -> &i32 {
        &self.gamma
    }

    pub fn render_distance(&self) -> &u8 {
        &self.render_distance
    }

    pub fn new(volume: i32, gamma: i32, render_distance: u8) -> Self {
        GameConfig {
            volume,
            gamma,
            render_distance,
        }
    }
}

impl<O: CodecOps> DefaultCodec<O> for GameConfig {
    fn codec() -> impl datafix::serialization::Codec<Self, O> {
        MapCodecBuilder::new()
            .field(i32::codec().field_of("volume", GameConfig::volume))
            .field(i32::codec().field_of("gamma", GameConfig::gamma))
            .field(u8::codec().field_of("render_distance", GameConfig::render_distance))
            .build(GameConfig::new)
    }
}

fn main() -> CodecResult<()> {
    let config = GameConfig::new(100, 50, 12);
    println!("{:?}", config);
    let mut encoded = GameConfig::codec().encode_start(&JsonOps, &config)?;
    println!("{}", encoded);
    encoded.insert("wrender_distance", "ok").unwrap();
    let decoded = GameConfig::codec().decode_start(&JsonOps, &encoded)?;
    println!("{:?}", decoded);

    assert_eq!(config, decoded);

    Ok(())
}

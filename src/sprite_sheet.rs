use bevy::prelude::*;
use bevy_ecs_tilemap::tiles::TileTextureIndex;

#[derive(Resource)]
pub struct SpriteSheet {
    image: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
}

impl FromWorld for SpriteSheet {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let image = asset_server.load("sprites.png");

        let mut layouts = world.resource_mut::<Assets<TextureAtlasLayout>>();

        let layout = TextureAtlasLayout::from_grid(UVec2::splat(32), 10, 10, None, None);
        let layout = layouts.add(layout);

        Self { image, layout }
    }
}

impl SpriteSheet {
    pub fn image(&self) -> Handle<Image> {
        self.image.to_owned()
    }

    pub fn sprite(&self, game_sprite: GameSprite) -> Sprite {
        Sprite::from_atlas_image(self.image.clone(), self.texture_atlas(game_sprite))
    }

    pub fn texture_atlas(&self, game_sprite: GameSprite) -> TextureAtlas {
        TextureAtlas {
            layout: self.layout.clone(),
            index: game_sprite.index(),
        }
    }
}

pub enum GameSprite {
    _JankyPlayer,
    _JankyPlayerThrust,
    Player,
    PlayerThrust0,
    PlayerThrust1,
    _Laser,
    Enemy,
    ConveyorInWOutE,
    ConveyorInSWOutE,
    ConveyorInSOutE,
    ConveyorInNSOutE,
    ConveyorInNSWOutE,
    BlankSquare,
    Delete,
    Arrow,
    Generator,
    Sink,
    Distributor,
    Bridge,
    OperatorPlus,
    OperatorMultiply,
}

impl GameSprite {
    pub fn index(&self) -> usize {
        use GameSprite::*;
        match self {
            _JankyPlayer => 0,
            _JankyPlayerThrust => 1,
            Player => 2,
            PlayerThrust0 => 3,
            PlayerThrust1 => 4,
            _Laser => 5,
            Enemy => 6,
            ConveyorInWOutE => 11,
            ConveyorInSWOutE => 12,
            ConveyorInSOutE => 13,
            ConveyorInNSOutE => 14,
            ConveyorInNSWOutE => 15,
            BlankSquare => 20,
            Delete => 21,
            Arrow => 22,
            Generator => 30,
            Sink => 31,
            Distributor => 32,
            Bridge => 33,
            OperatorPlus => 34,
            OperatorMultiply => 35,
        }
    }

    pub fn tile_texture_index(&self) -> TileTextureIndex {
        TileTextureIndex(self.index() as u32)
    }

    pub fn player_thrust(n: u32) -> GameSprite {
        match n % 2 {
            0 => GameSprite::PlayerThrust0,
            1 => GameSprite::PlayerThrust1,
            _ => panic!(),
        }
    }
}

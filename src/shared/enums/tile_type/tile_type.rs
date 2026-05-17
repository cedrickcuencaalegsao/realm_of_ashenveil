#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TileType {
    DeepWater, // height < -8
    Water,     // -8 <= height < -2
    Sand,      // -2 <= height < 0
    Grass,     //  0 <= height < 6
    Dirt,      //  6 <= height < 10
    Stone,     // 10 <= height < 13
    Snow,      // 13 <= height <= 15
}

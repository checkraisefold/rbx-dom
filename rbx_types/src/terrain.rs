use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashMap},
};

use crate::Vector3;

/// Coordinates of a chunk or a voxel. For internal use.
// Can't use Vector3int16; we need a 32 bit integer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct TerrainVec {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl TerrainVec {
    pub fn from_vec3(i: Vector3) -> Self {
        Self {
            x: i.x as i32,
            y: i.y as i32,
            z: i.z as i32,
        }
    }

    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

/// Coordinates of a `Voxel` inside of a `Chunk`, which is a grid of 4 units in world space.
/// This is inside of a grid of 32^3 voxels per chunk.
#[repr(transparent)]
#[derive(Debug, PartialEq, Eq, Default, Clone, Copy, Hash)]
pub struct VoxelCoordinates(TerrainVec);

impl VoxelCoordinates {
    /// Constructs a new `VoxelCoordinates` object.
    #[inline]
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self(TerrainVec::new(x, y, z))
    }

    /// Constructs a new `VoxelCoordinates` object from a Vector3.
    #[inline]
    pub fn from_vec3(i: Vector3) -> Self {
        Self(TerrainVec::from_vec3(i))
    }
}

/// Coordinates of a `Chunk` in chunk space, which is a grid of 128 units in world space.
/// Relevant for usage with a `Terrain` object. Inside a grid of 524,288^3 chunks per world.
#[repr(transparent)]
#[derive(Debug, PartialEq, Eq, Default, Clone, Copy, Hash)]
pub struct ChunkCoordinates(TerrainVec);

impl ChunkCoordinates {
    /// Constructs a new `ChunkCoordinates` object.
    #[inline]
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self(TerrainVec::new(x, y, z))
    }

    /// Constructs a new `ChunkCoordinates` object from a Vector3.
    #[inline]
    pub fn from_vec3(i: Vector3) -> Self {
        Self(TerrainVec::from_vec3(i))
    }
}

impl Ord for VoxelCoordinates {
    fn cmp(&self, other: &Self) -> Ordering {
        let x_cmp = self.0.x.cmp(&other.0.x);
        let y_cmp = self.0.y.cmp(&other.0.y);
        let z_cmp = self.0.z.cmp(&other.0.z);

        match (y_cmp == Ordering::Equal, z_cmp == Ordering::Equal) {
            (true, true) => x_cmp,
            (true, false) => z_cmp,
            (false, false) => y_cmp,
            _ => y_cmp,
        }
    }
}

impl Ord for ChunkCoordinates {
    fn cmp(&self, other: &Self) -> Ordering {
        let x_cmp = self.0.x.cmp(&other.0.x);
        let y_cmp = self.0.y.cmp(&other.0.y);
        let z_cmp = self.0.z.cmp(&other.0.z);

        match (x_cmp == Ordering::Equal, y_cmp == Ordering::Equal) {
            (true, true) => z_cmp,
            (true, false) => y_cmp,
            (false, false) => x_cmp,
            _ => x_cmp,
        }
    }
}

impl PartialOrd for VoxelCoordinates {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialOrd for ChunkCoordinates {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[repr(u8)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Material {
    #[default]
    Air,
    Water,
    Grass,
    Slate,
    Concrete,
    Brick,
    Sand,
    WoodPlanks,
    Rock,
    Glacier,
    Snow,
    Sandstone,
    Mud,
    Basalt,
    Ground,
    CrackedLava,
    Asphalt,
    Cobblestone,
    Ice,
    LeafyGrass,
    Salt,
    Limestone,
    Pavement,
}

trait TerrainSerializer {
    fn encode(&self) -> Vec<u8>;
    //fn decode(&self) -> Self;
}

/// A container for a voxel of terrain, used in the `Chunk` object.
#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub struct Voxel {
    solid_occupancy: f32,
    water_occupancy: f32,
    pub material: Material,
}

impl Voxel {
    /// Constructs a new `Voxel` with a material and occupancy percentage.
    /// Equivalent to data writeable from Roblox's `Terrain:WriteVoxels`.
    /// Occupancy values are between `0.0` and `1.0`, as a percentage of the voxel.
    pub fn new(material: Material, solid_occupancy: f32) -> Self {
        let mut voxel = Self {
            material,
            ..Default::default()
        };
        voxel.set_occupancy(solid_occupancy, 0.0);

        voxel
    }

    /// Constructs a new `Voxel` with a material, solid occupancy, and water
    /// occupancy percentage. Takes advantage of the recently-released
    /// Shorelines feature. Equivalent to data writeable from Roblox's `Terrain:WriteVoxelChannels`.
    /// Occupancy values are between `0.0` and `1.0`, as a percentage of the voxel.
    pub fn new_with_water(material: Material, solid_occupancy: f32, water_occupancy: f32) -> Self {
        let mut voxel = Self {
            material,
            ..Default::default()
        };
        voxel.set_occupancy(solid_occupancy, water_occupancy);

        voxel
    }

    fn get_encode_data(&self) -> (u8, u8) {
        let solid_occupancy: u8 = (self.solid_occupancy * 255.0) as u8;
        let water_occupancy: u8 = (self.water_occupancy * 255.0) as u8;
        (solid_occupancy, water_occupancy)
    }

    fn encode_run_length(&self, count: u16) -> Vec<u8> {
        let (solid_occupancy, water_occupancy) = self.get_encode_data();
        let mut flag = self.material as u8;
        let mut to_write: Vec<u8> = vec![];

        if solid_occupancy != 0xFF && solid_occupancy != 0x00 {
            // Should we store the solid occupancy value?
            flag |= 0b01000000;
            to_write.push(solid_occupancy);
        }
        if count > 1 {
            // Should we store the count (amount of voxels this run length) value?
            flag |= 0b10000000;
            if water_occupancy == 0 {
                to_write.push((count - 1) as u8);
            } else {
                to_write.push(0);
            }
        }
        to_write.insert(0, flag);

        if water_occupancy != 0 && count > 1 {
            /* Shorelines uses a new water occupancy value in the voxel data. Because of this,
            Roblox uses a hack to avoid having to reduce their 6 bits of material ID freedom
            by writing voxels with a count bit set to 1 and no count. This means we have to write
            all voxels in the run length manually. */
            return to_write
                .iter()
                .cycle()
                .copied()
                .take(to_write.len() * count as usize)
                .collect();
        }
        to_write
    }

    /// Sets occupancy data for a `Voxel`. Water occupancy is from the
    /// new Shorelines feature. Occupancy values are between `0.0` and `1.0`,
    /// as a percentage of the voxel.
    pub fn set_occupancy(&mut self, solid_occupancy: f32, water_occupancy: f32) {
        self.solid_occupancy = if self.material == Material::Air {
            1.0
        } else {
            solid_occupancy.clamp(0.0, 1.0)
        };

        // Full with a solid (non-air) material? We can't have any water.
        if self.material != Material::Air && solid_occupancy == 1.0 && water_occupancy > 0.0 {
            self.water_occupancy = 0.0
        } else {
            self.water_occupancy = water_occupancy.clamp(0.0, 1.0);
        }
    }
}

// We don't iterate over the grid, so we can use a HashMap instead of BTreeMap to save performance.
/// A container for a chunk of terrain, used in the `Terrain` object.
#[derive(Default, Clone)]
pub struct Chunk {
    grid: HashMap<VoxelCoordinates, Voxel>,
    /// For all empty voxels in the chunk, we will write this material
    /// at 100% occupancy. Defaults to `Material::Air`.
    pub base_material: Material,
}

impl Chunk {
    /// Constructs a new `Chunk` with a base material of `Material::Air`.
    #[inline]
    pub fn new() -> Self {
        Self {
            grid: HashMap::new(),
            base_material: Material::Air,
        }
    }

    /// Constructs a new `Chunk` with a user-provided base material.
    #[inline]
    pub fn new_with_base(base_material: Material) -> Self {
        Self {
            grid: HashMap::new(),
            base_material,
        }
    }

    /// Finds a `Voxel` at the given position in this `Chunk`,
    /// returning a reference to it if it exists.
    #[inline]
    pub fn get_voxel(&self, position: &VoxelCoordinates) -> Option<&Voxel> {
        self.grid.get(position)
    }

    /// Finds a `Voxel` at the given position in this `Chunk`,
    /// returning a mutable reference to it if it exists.
    #[inline]
    pub fn get_voxel_mut(&mut self, position: &VoxelCoordinates) -> Option<&mut Voxel> {
        self.grid.get_mut(position)
    }

    /// Writes (or overwrites) a `Chunk` at the given position to this `Terrain`.
    #[inline]
    pub fn write_voxel(&mut self, position: &VoxelCoordinates, voxel: Voxel) {
        self.grid.insert(*position, voxel);
    }
}

impl TerrainSerializer for Chunk {
    fn encode(&self) -> Vec<u8> {
        // ~256 bytes if all voxels are air/base mat with maximum count. Double it
        let mut data = Vec::<u8>::with_capacity(512);

        let base_voxel: Voxel = Voxel {
            solid_occupancy: 1.0,
            water_occupancy: 0.0,
            material: self.base_material,
        };

        let mut pos_cursor = VoxelCoordinates::default();
        let mut run_length_cursor = (0u16, &base_voxel);
        for x in 0..32 {
            pos_cursor.0.x = x;
            for y in 0..32 {
                pos_cursor.0.y = y;
                for z in 0..32 {
                    pos_cursor.0.z = z;

                    let grabbed_voxel = match self.grid.get(&pos_cursor) {
                        Some(v) => v,
                        _ => &base_voxel,
                    };

                    if run_length_cursor.0 == 0 {
                        // We don't add 1 here, next if statement does it.
                        run_length_cursor.1 = grabbed_voxel;
                    }
                    if grabbed_voxel == run_length_cursor.1 && run_length_cursor.0 < 0xFF {
                        run_length_cursor.0 += 1;
                        continue;
                    } else if run_length_cursor.0 >= 0xFF {
                        // To a count of 256 for encoding.
                        run_length_cursor.0 += 1;
                    }

                    data.extend(grabbed_voxel.encode_run_length(run_length_cursor.0));
                    run_length_cursor.0 = 1;
                    run_length_cursor.1 = grabbed_voxel;
                }
            }
        }

        data
    }
}

/// A container allowing the modification, encoding, and decoding of the
/// `SmoothGrid` data used by Roblox's `Terrain` object.
#[derive(Default)]
pub struct Terrain {
    world: BTreeMap<ChunkCoordinates, Chunk>,
}

impl Terrain {
    /// Constructs a new `Terrain` with no chunks.
    #[inline]
    pub fn new() -> Self {
        Self {
            world: BTreeMap::new(),
        }
    }

    /// Finds a `Chunk` at the given position in this `Terrain`,
    /// returning a reference to it if it exists.
    #[inline]
    pub fn get_chunk(&self, position: &ChunkCoordinates) -> Option<&Chunk> {
        self.world.get(position)
    }

    /// Finds a `Chunk` at the given position in this `Terrain`,
    /// returning a mutable reference to it if it exists.
    #[inline]
    pub fn get_chunk_mut(&mut self, position: &ChunkCoordinates) -> Option<&mut Chunk> {
        self.world.get_mut(position)
    }

    /// Writes (or overwrites) a `Chunk` at the given position in this `Terrain`.
    #[inline]
    pub fn write_chunk(&mut self, position: &ChunkCoordinates, chunk: Chunk) {
        self.world.insert(*position, chunk);
    }
}

impl TerrainSerializer for Terrain {
    fn encode(&self) -> Vec<u8> {
        let mut data = Vec::<u8>::with_capacity(self.world.len() * 512);
        data.extend([0x01, 0x05]);

        let mut chunk_cursor: Option<&ChunkCoordinates> = None;
        for (position, chunk) in self.world.iter() {
            let cursor = match chunk_cursor {
                None => position,
                Some(c) => c,
            };
            let axes = [
                position.0.x - cursor.0.x,
                position.0.y - cursor.0.y,
                position.0.z - cursor.0.z,
            ];

            let mut negative_padding = 3;
            let mut negative_axes = [0x00, 0x00, 0x00];
            let mut adjusted_axes = [[0x00, 0x00, 0x00], [0x00, 0x00, 0x00], [0x00, 0x00, 0x00]];
            for (key, axis) in axes.iter().enumerate() {
                if *axis < 0 {
                    negative_axes[key] = 0xFF as u8;
                }

                let axis_filler = match axis.abs() {
                    ..256 => 3,
                    ..65536 => 2,
                    65536.. => 1,
                };
                if axis_filler < negative_padding {
                    negative_padding = axis_filler;
                }

                // FIXME: This is really ugly
                let mut axis_adjuster = axis.abs();
                while axis_adjuster > 0 {
                    match axis_adjuster {
                        ..256 => {
                            adjusted_axes[2][key] = axis_adjuster as u8;
                            axis_adjuster -= axis_adjuster;
                        }
                        ..65536 => {
                            let offset = axis_adjuster / 256;
                            adjusted_axes[1][key] += offset as u8;
                            axis_adjuster -= axis_adjuster * offset;
                        }
                        65536.. => {
                            let offset = axis_adjuster / 65536;
                            adjusted_axes[0][key] += offset as u8;
                            axis_adjuster -= axis_adjuster * offset;
                        }
                    }
                }
            }

            for _ in 0..negative_padding {
                data.extend(negative_axes.iter())
            }

            // 3 -> 1, 2 -> 2, 1 -> 3. Amount of 256 multiples to write
            for i in 0..(4 - negative_padding) {
                data.extend(adjusted_axes[2 - i].iter());
            }

            data.extend(chunk.encode());
            chunk_cursor = Some(position);
        }

        data
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn encode_default() {
        let mut terr = Terrain::new();
        let chunk = Chunk::new_with_base(Material::Grass);
        terr.write_chunk(&ChunkCoordinates::default(), chunk.clone());
        terr.write_chunk(&ChunkCoordinates::new(1, 0, 0), chunk.clone());

        let encoded = base64::encode(terr.encode());
        println!("{}", encoded);
    }
}

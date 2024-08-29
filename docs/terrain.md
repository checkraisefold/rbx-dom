# Roblox Terrain Binary Format

This document describes the Terrain binary format. In this format there is no field for Version, so it is assumed that any future changes will be additions to the existing format or a new format entirely. This specification does not include the adjacent PhysicsGrid binary format, only SmoothGrid.

# Contents

- [Document Conventions](#document-conventions)
- [File Structure](#file-structure)
- [Data Types](#data-types)
  - [Chunk](#chunk)
  - [Voxel](#voxel)
  - [Voxel.Flag](#voxel.flag)
  - [Material](#material)

## Document Conventions

This document assumes a basic understanding of Rust's convention for numeric types. For example:

- `u32` is an unsigned 32-bit integer
- `f32` is a 32-bit floating point number

All numeric types are little endian. Floats are stored as a dividend of their type's numeric maximum; as such, a `f8` with a value of `0x31` translates to `0x31 / 0xFF`, or `~0.192`. Floats are unsigned. Signed integers are stored with their absolute value, with preceding bytes containing the signedness information; meaning the range for an `i8` is `-0xFF` -> `0xFF`, instead of `-0x7F` to `0x7F`.

Unless otherwise noted, all structs in this document are assumed to be stored with their components in the sequence listed without any modification. That is, if a struct is listed as being composed of an `i32` and an `f32`, it can be assumed that it is stored as an `i32` followed by an `f32`.

## File Structure

The first two bytes of the blob are `0x01`, which is a magic number, followed by `0x05`, which is a logarithm base 2 of the chunk size in voxels. Currently, the chunk size is 32<sup>3</sup> (32768). Values other than `0x05` are untested.

Immediately following the header is an array of chunks, each of which must have a full voxel count, which is currently 32<sup>3</sup>. Chunks are ascendingly ordered by X, then Y, then Z based on their position in the world. Each chunk represents a cube of 128<sup>3</sup> units in world space.

| Field Name | Format          | Value                                                                 |
| :--------- | :-------------- | :-------------------------------------------------------------------- |
| Header     | `[u8; 2]`       | Magic number and chunk size.                                          |
| Chunks     | [Chunk](#chunk) | As many chunks as needed to fill the blob. Unknown/no maximum amount. |

## Data Types

Terrain data is represented using a variety of different data types. Data described within them follows any conventions set in the [document conventions](#document-conventions).

### Chunk

The `Chunk` type is stored with a dynamic size dependent on the size of the voxels contained within, and the end of its data is marked by reaching its maximum voxel count of 32<sup>3</sup>. Voxels are ascendingly ordered by Y, then Z, then X based on their position in the chunk. Voxels are stored in rows of 32 units on each axis, each representing 4 units in world space.

Voxel data is preceded by the offset between this chunk and the last in the blob, or from `0, 0, 0` if this is the first chunk in the blob. This offset is stored using 3 vectors of `(x: u8, y: u8, z: u8)` using the signedness determination described in [document conventions](#document-conventions), with 0xFF values in _all_ unused offsets indicating a negative sign. All coordinates are in chunk space (increments of 1 chunk), not world space.

For example, given two chunks, one at a position of `(2, 0, 0)` and another at a position of `(4, 0, -1)` in chunk space, the latter's offset would be stored as follows:

1. (0x00, 0x00, 0xFF)
2. (0x00, 0x00, 0xFF)
3. (0x00, 0x00, 0xFF)
4. (0x02, 0x00, 0x01)

| Field Name     | Format          | Value                                                                               |
| :------------- | :-------------- | :---------------------------------------------------------------------------------- |
| Signedness     | `[u8; 3]`       | Always the signedness of the following vectors. 0xFF in negative axes.              |
| Offset (65536) | `[i8; 3]`       | Offset from the last chunk in the blob, each axis being multiplied by 65536 chunks. |
| Offset (256)   | `[i8; 3]`       | Offset from the last chunk in the blob, each axis being multiplied by 256 chunks.   |
| Offset (1)     | `[i8; 3]`       | Offset from the last chunk in the blob, each axis being in singular chunks.         |
| Voxels         | [Voxel](#voxel) | As many voxels as needed to fulfill the maximum amount per chunk.                   |

### Voxel

The `Voxel` type is stored with a dynamic size (between 1-4 bytes) dependent on its set [bitflags](#voxel.flag). Only stored within [chunks](#chunk). A reference for the data contained within voxels [can be found here][Terrain.WriteVoxelChannels]. Empty voxels are stored with their material set to Air.

Voxels are stored within a chunk using [run-length encoding]. A count of `1` would indicate the voxel repeating itself once, meaning 2 of the same voxel in a row would be read when decoded. Occupancy values are stored using the float format described in the [document conventions](#document-conventions).

Water occupancy was added as an addendum to the existing format for the new [Shorelines] feature release. To avoid restricting the [Voxel.Flag](#voxel.flag)'s `Material Index` field to 5 bits, or rearchitecting the format, run-length encoding is disabled for the current voxel when it has a water occupancy of above `0.0`. Instead, the `Store Count` bit is enabled, and a count of `0` is written. Following the empty count, the water occupancy byte is stored. If a voxel has a solid occupancy of `1.0` (unless the material is Air), the water occupancy should always be `0.0`.

| Field Name      | Format                    | Value                                                                                                          |
| :-------------- | :------------------------ | :------------------------------------------------------------------------------------------------------------- |
| Flag            | [Voxel.Flag](#voxel.flag) | Contains the material of this voxel, along with other bitflags.                                                |
| Solid Occupancy | `f8`                      | Occupancy between 0-100% of the set material for this voxel. Only stored if the `Store Occupancy` flag is set. |
| Count           | `u8`                      | Run-length count. Only stored if the `Store Count` flag is set.                                                |
| Water Occupancy | `f8`                      | Occupancy between 0-100% of Water for this voxel. Only stored based on the conditions described above.         |

[run-length encoding]: https://en.wikipedia.org/wiki/Run-length_encoding
[Terrain.WriteVoxelChannels]: https://create.roblox.com/docs/reference/engine/classes/Terrain#WriteVoxelChannels
[Shorelines]: https://devforum.roblox.com/t/shorelines-full-release/2952103

### Voxel.Flag

The `Voxel.Flag` subtype is a 1-byte (unsigned) bitflag describing the data written to a voxel. The occupancy bit is only set if the voxel's solid occupancy is not `1.0`. The count bit is only set if the voxel's water occupancy is not `0.0`, or the voxel has a run-length count of `1` or above.

The following description is in order from least to most significant bits.

| Flag Name       | Bits | Value                                                                                  |
| :-------------- | :--- | :------------------------------------------------------------------------------------- |
| Material Index  | 6    | Integer index of this voxel's material. See the [Material](#material) enum for values. |
| Store Occupancy | 1    | Boolean for whether we should store this voxel's solid occupancy as a byte.            |
| Store Count     | 1    | Boolean for whether we should store this voxel's run-length count as a byte.           |

### Material

The `Material` enum is used for the `Material Index` value of a [Voxel.Flag](#voxel.flag) to set the material of a [Voxel](#voxel). Constitutes a terrain-specific subset of Roblox's [Enum.Material].

| Material    | Value |
| :---------- | :---- |
| Air         | 0     |
| Water       | 1     |
| Grass       | 2     |
| Slate       | 3     |
| Concrete    | 4     |
| Brick       | 5     |
| Sand        | 6     |
| WoodPlanks  | 7     |
| Rock        | 8     |
| Glacier     | 9     |
| Snow        | 10    |
| Sandstone   | 11    |
| Mud         | 12    |
| Basalt      | 13    |
| Ground      | 14    |
| CrackedLava | 15    |
| Asphalt     | 16    |
| Cobblestone | 17    |
| Ice         | 18    |
| LeafyGrass  | 19    |
| Salt        | 20    |
| Limestone   | 21    |
| Pavement    | 22    |

[Enum.Material]: https://create.roblox.com/docs/reference/engine/enums/Material

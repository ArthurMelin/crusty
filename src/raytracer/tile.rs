use std::cmp::{max, min};
use std::mem::swap;

pub struct Tile {
    pub left: u32,
    pub right: u32,
    pub top: u32,
    pub bottom: u32,
}

fn divide_up(a: u32, b: u32) -> u32 {
    (a + b - 1) / b
}

pub fn hilbert_tiles(width: u32, height: u32, tile_sz: u32) -> Vec<Tile> {
    // Generate tiles using the Hilbert Spiral algorithm from Blender's Cycles engine
    // https://github.com/blender/blender/blob/blender-v2.93-release/intern/cycles/render/tile.cpp#L198

    // Size of blocks in tiles, must be a power of 2
    let hilbert_sz = if tile_sz <= 12 { 8 } else { 4 };
    let block_sz = tile_sz * hilbert_sz;

    // Number of tiles to fill the output
    let tile_cnt = (
        if tile_sz >= width { 1 } else { divide_up(width, tile_sz) },
        if tile_sz >= height { 1 } else { divide_up(height, tile_sz) },
    );

    // Number of blocks to fill the output
    let block_cnt = (
        if block_sz >= width { 1 } else { divide_up(width, block_sz) },
        if block_sz >= height { 1 } else { divide_up(height, block_sz) },
    );

    // Allocate result vector
    let mut tiles = Vec::<Tile>::with_capacity((tile_cnt.0 * tile_cnt.1) as usize);

    // Side length of the spiral (must be odd)
    let n = max(block_cnt.0, block_cnt.1) | 0x1;

    // Offset of spiral (to keep it centered)
    let offset = (
        (width as i32 - (n * block_sz) as i32) / 2 / tile_sz as i32 * tile_sz as i32,
        (height as i32 - (n * block_sz) as i32) / 2 / tile_sz as i32 * tile_sz as i32,
    );

    let mut block = (0, 0);
    let mut dir = 0;
    let mut prev_dir = 0;
    let mut i = 0;
    loop {
        // Generate the tiles in the current block
        for hilbert_index in 0..hilbert_sz * hilbert_sz {
            let hilbert_pos = hilbert_index_to_position(hilbert_sz, hilbert_index);

            // Rotate block according to spiral direction
            let tile = if dir == 0 && prev_dir == 0 {
                (hilbert_pos.1, hilbert_pos.0)
            } else if dir == 1 || prev_dir == 1 {
                (hilbert_pos.0, hilbert_pos.1)
            } else if dir == 2 {
                (hilbert_sz - 1 - hilbert_pos.1, hilbert_sz - 1 - hilbert_pos.0)
            } else {
                (hilbert_sz - 1 - hilbert_pos.0, hilbert_sz - 1 - hilbert_pos.1)
            };

            // Push tile to queue (if it's in the output)
            let tile_pos = (
                (block.0 * block_sz + tile.0 * tile_sz) as i32 + offset.0,
                (block.1 * block_sz + tile.1 * tile_sz) as i32 + offset.1,
            );
            if  tile_pos.0 >= 0 && tile_pos.0 < width as i32 &&
                tile_pos.1 >= 0 && tile_pos.1 < height as i32 {
                let tile_pos = (tile_pos.0 as u32, tile_pos.1 as u32);
                tiles.push(Tile {
                    left: tile_pos.0,
                    top: tile_pos.1,
                    right: tile_pos.0 + min(tile_sz, width - tile_pos.0),
                    bottom: tile_pos.1 + min(tile_sz, height - tile_pos.1),
                });
            }
        }

        // Stop when the spiral has reached the center
        if block.0 == (n - 1) / 2 && block.1 == (n - 1) / 2 {
            break;
        }

        // Advance to next block
        prev_dir = dir;
        match dir {
            0 => {
                block.1 += 1;
                if block.1 == n - i - 1 {
                    dir += 1;
                }
            }
            1 => {
                block.0 += 1;
                if block.0 == n - i - 1 {
                    dir += 1;
                }
            }
            2 => {
                block.1 -= 1;
                if block.1 == i {
                    dir += 1;
                }
            }
            3 => {
                block.0 -= 1;
                if block.0 == i + 1 {
                    dir = 0;
                    i += 1;
                }
            }
            _ => unreachable!(),
        }
    }

    tiles
}

fn hilbert_index_to_position(n: u32, mut d: u32) -> (u32, u32) {
    // Convert Hilbert index to position using black magic
    let mut s = 1;
    let mut r = (0, 0);
    let mut xy = (0, 0);
    while s < n {
        r.0 = (d >> 1) & 1;
        r.1 = (d ^ r.0) & 1;
        if r.1 == 0 {
            if r.0 != 0 {
                xy = (s - 1 - xy.0, s - 1 - xy.1);
            }
            swap(&mut xy.0, &mut xy.1);
        }
        xy = (xy.0 + r.0 * s, xy.1 + r.1 * s);
        d >>= 2;
        s *= 2;
    }
    xy
}

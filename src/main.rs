use std::ffi::{CStr, CString};
use std::ptr::null_mut;
use std::time;
use std::time::{Instant, SystemTime};
use citro2d::render::C2DTarget;
use citro2d_sys::{C2D_Color32f, C2D_DrawRectangle, C2D_DrawText, C2D_FontFree, C2D_FontLoadSystem, C2D_SceneBegin, C2D_TextBufDelete, C2D_TextBufNew, C2D_TextFontParse, C2D_TextOptimize, C2D_TextParse};
use citro3d::render::{ClearFlags, Target};
use ctru::prelude::*;
use ctru::services::cfgu::Cfgu;
use ctru::services::gfx::{RawFrameBuffer, Screen};
use picorand::{PicoRandGenerate, RNG, WyRand};

const DEFAULT_CONFIGS: [BoardConfig; 3] = [
    BoardConfig {
        name: "Easy",
        width: 8,
        height: 10,
        mines: 10,
    },
    BoardConfig {
        name: "Medium",
        width: 14,
        height: 18,
        mines: 40,
    },
    BoardConfig {
        name: "Difficult",
        width: 20,
        height: 24,
        mines: 99,
    }
];

#[derive(Clone)]
enum State {
    Revealed,
    Flag,
    Covered
}

#[derive(Clone)]
enum TileType {
    Blank(u8),
    Mine
}

#[derive(Clone)]
struct Tile {
    tile_type: TileType,
    state: State
}

impl Tile {
    pub fn blank() -> Tile {
        Tile {
            tile_type: TileType::Blank(0),
            state: State::Covered
        }
    }

    pub fn mark_neighbor(&mut self, inc: bool) {
        if let TileType::Blank(num) = self.tile_type {
            if inc {
                self.tile_type = TileType::Blank(num + 1);
            } else if num > 0 {
                self.tile_type = TileType::Blank(num - 1);
            }
        }
    }
}

#[derive(Clone)]
struct BoardConfig {
    name: &'static str,
    width: u8,
    height: u8,
    mines: u8
}

impl BoardConfig {
    // pub fn new(name: String, width: u8, height: u8, mines: u8) -> Result<Self, String> {
    //     let tile_amount = width * height;
    //
    //     if mines >= tile_amount {
    //         Err(format!("Number of mines can't be equal to or higher than number of tiles, found {} mines for {} tiles", mines, tile_amount))
    //     } else {
    //         Ok(BoardConfig {
    //             name: name.as_str(),
    //             width,
    //             height,
    //             mines,
    //         })
    //     }
    // }
}

struct Board {
    config: BoardConfig,
    board: Vec<Vec<Tile>>
}

impl Board {
    pub fn new(config: BoardConfig) -> Self {
        let width = config.width.clone();
        let height = config.height.clone();
        Board {
            config,
            board: vec![
                vec![
                    Tile::blank();
                    height as usize
                ];
                width as usize
            ],
        }
    }

    pub fn place_mines(&mut self, seed: u64) {
        let width = self.config.width;
        let height = self.config.height;
        let mut mines = self.config.mines.clone();

        let mut rand_instance = RNG::<WyRand, u8>::new(seed);

        loop {
            if mines == 0 { break }

            let rand_x = rand_instance.generate_range(0, width as usize) as usize;
            let rand_y = rand_instance.generate_range(0, height as usize) as usize;

            if self.is_tile_in_board(rand_x, rand_y) && !self.is_mine(rand_x, rand_y) {
                if let Ok(_) = self.set_tile(rand_x, rand_y, TileType::Mine) {
                    let mut neighbors = vec![
                        (rand_x + 1, rand_y),
                        (rand_x, rand_y + 1),
                        (rand_x + 1, rand_y + 1)
                    ];

                    if rand_x != 0 && rand_y != 0 {
                        neighbors.push((rand_x - 1, rand_y - 1));
                    }

                    if rand_x != 0 && rand_y == 0 {
                        neighbors.push((rand_x - 1, rand_y));
                        neighbors.push((rand_x - 1, rand_y + 1));
                    }

                    if rand_y != 0 && rand_x == 0 {
                        neighbors.push((rand_x, rand_y - 1));
                        neighbors.push((rand_x + 1, rand_y - 1));
                    }

                    for neighbor in neighbors {
                        if let Some(tile) = self.get_tile_mut(neighbor.0, neighbor.1) {
                            tile.mark_neighbor(true);
                        }
                    }

                    mines -= 1;
                }
            }
        }
    }

    pub fn is_mine(&self, x: usize, y: usize) -> bool {
        if let Some(tile) = self.get_tile(x, y) {
            if let TileType::Mine = tile.tile_type {
                return true;
            }
        }

        false
    }

    pub fn is_tile_in_board(&self, x: usize, y: usize) -> bool {
        match self.get_tile(x, y) {
            None => false,
            Some(_) => true
        }
    }

    pub fn get_tile(&self, x: usize, y: usize) -> Option<&Tile> {
        if let Some(line) = self.board.get(x) {
            line.get(y)
        } else {
            None
        }
    }

    pub fn get_tile_mut(&mut self, x: usize, y: usize) -> Option<&mut Tile> {
        if let Some(line) = self.board.get_mut(x) {
            line.get_mut(y)
        } else {
            None
        }
    }

    pub fn set_tile(&mut self, x: usize, y: usize, tile_type: TileType) -> Result<String, String> {
        if let Some(tile) = self.get_tile_mut(x, y) {
            tile.tile_type = tile_type;
            Ok(format!("Set tile type of tile at {} {}", x, y))
        } else {
            Err(format!("Failed to set tile at {} {}", x, y))
        }
    }
}

fn main() {
    ctru::use_panic_handler();

    let apt = Apt::new().unwrap();
    let mut hid = Hid::new().unwrap();
    let gfx = Gfx::new().unwrap();
    let cfgu = Cfgu::new().unwrap();
    // let _console = Console::new(gfx.top_screen.borrow_mut());

    let cur_time = SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap().as_secs();

    let mut c3d_instance = citro3d::Instance::new().expect("failed to initialize Citro3D");
    let mut c2d_instance = citro2d::Instance::new().expect("failed to initialize Citro2D");

    c2d_instance.prepare();

    let mut top_screen = gfx.top_screen.borrow_mut();
    top_screen.set_wide_mode(true);
    let RawFrameBuffer { width, height, .. } = top_screen.raw_framebuffer();

    let mut top_target = Target::new_c2d(top_screen);

    let mut bottom_screen = gfx.bottom_screen.borrow_mut();
    let mut bottom_target = Target::new_c2d(bottom_screen);

    // let font = unsafe {
    //     C2D_FontLoadSystem(1)
    // };

    let mut board = Board::new(DEFAULT_CONFIGS[0].clone());
    board.place_mines(cur_time);

    // let cell_width_total = width / (board.config.width as usize);
    // let cell_height_total = (height / 2) / (board.config.height as usize);
    let cell_width_total = 30;
    let cell_height_total = 30;
    let pad: usize = 5;

    let mut mines_left = board.config.mines.clone();

    let mut rand_instance = RNG::<WyRand, u32>::new(cur_time);

    // let texts = [
    //     (format!("Size: {} by {}", board.config.height, board.config.width), 1, 1),
    //     (format!("Mines lefts: {}/{}", mines_left, board.config.mines), 1, 1)
    // ];
    //
    // let mut text_vec = vec![];
    //
    // let text_buf = unsafe {C2D_TextBufNew(4096)};
    //
    // for text in texts {
    //     unsafe {
    //         let text_ptr = null_mut();
    //         let string = CString::new(text.0.as_str()).expect("Failed to parse &str");
    //         let cstr = CStr::from_bytes_with_nul_unchecked(string.to_bytes_with_nul());
    //
    //         C2D_TextFontParse(text_ptr,
    //                       font,
    //                       text_buf, cstr.as_ptr());
    //
    //         C2D_TextOptimize(text_ptr);
    //
    //         text_vec.push((text_ptr, text.1, text.2));
    //     }
    // }

    while apt.main_loop() {
        gfx.wait_for_vblank();

        hid.scan_input();

        if hid.keys_down().contains(KeyPad::START) {
            break;
        }

        c3d_instance.render_frame_with(|instance| {
            let mut render_to = |target: &mut Target, top: bool| {
                c2d_instance.select_render_target(target);
                target.clear_c2d(0);

                if top {
                    for i in 0..board.config.width {
                        let i: usize = i as usize;
                        let x = i * cell_width_total;
                        let min_x = x + pad;
                        let max_x = x + cell_width_total - pad;
                        let x_width = max_x - min_x;

                        for j in 0..board.config.height {
                            let j = j as usize;
                            let y = j * cell_height_total;
                            let min_y = y + pad;
                            let max_y = y + cell_height_total - pad;
                            let y_height = max_y - min_y;

                            let mut color: u32 = unsafe {
                                C2D_Color32f(255.0f32, 255.0f32, 255.0f32, 1.0f32)
                            };

                            if board.is_mine(i, j) {
                                color = unsafe {
                                    C2D_Color32f(255.0f32, 0.0f32, 0.0f32, 1.0f32)
                                };
                            }

                            unsafe {
                                C2D_DrawRectangle(
                                    min_x as f32,
                                    min_y as f32,
                                    0f32,
                                    x_width as f32,
                                    y_height as f32,
                                    color,
                                    color,
                                    color,
                                    color
                                );
                            }
                        }
                    }
                } else {
                    // for text in text_vec.clone() {
                    //     unsafe {
                    //         C2D_DrawText(
                    //             text.0,
                    //             0,
                    //             text.1 as f32,
                    //             text.2 as f32,
                    //             0.0f32,
                    //             1.0f32,
                    //             1.0f32
                    //         );
                    //     }
                    // }
                }
            };

            render_to(&mut top_target, true);
            render_to(&mut bottom_target, false);
        })
    }

    // unsafe {
    //     C2D_TextBufDelete(text_buf);
    //     C2D_FontFree(font);
    // }

    // unsafe {
    //
    // }
}

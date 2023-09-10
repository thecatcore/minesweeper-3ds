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
use crate::State::{Covered, Revealed};
use crate::TileType::Blank;

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

impl TileType {
    fn get_color(&self) -> u32 {
        unsafe {
            match self {
                Blank(num) => {
                    match num {
                        1 => C2D_Color32f(0.0f32, 0.0f32, 255.0f32, 1.0f32),
                        2 => C2D_Color32f(0.0f32, 255.0f32, 0.0f32, 1.0f32),
                        3 => C2D_Color32f(255.0f32, 0.0f32, 0.0f32, 1.0f32),
                        4 => C2D_Color32f(180.0f32, 0.0f32, 255.0f32, 1.0f32),
                        5 => C2D_Color32f(154.0f32, 135.0f32, 111.0f32, 1.0f32),
                        6 => C2D_Color32f(0.0f32, 255.0f32, 235.0f32, 1.0f32),
                        7 => C2D_Color32f(0.0f32, 0.0f32, 0.0f32, 1.0f32),
                        8 => C2D_Color32f(135.0f32, 135.0f32, 135.0f32, 1.0f32),
                        _ => C2D_Color32f(255.0f32, 255.0f32, 255.0f32, 1.0f32)
                    }
                }
                TileType::Mine => C2D_Color32f(0.0f32, 0.0f32, 0.0f32, 1.0f32)
            }
        }
    }
}

#[derive(Clone)]
struct Tile {
    tile_type: TileType,
    state: State
}

impl Tile {
    pub fn blank() -> Tile {
        Tile {
            tile_type: Blank(0),
            state: State::Covered
        }
    }

    pub fn mark_neighbor(&mut self, inc: bool) {
        if let TileType::Blank(num) = self.tile_type {
            if inc {
                self.tile_type = Blank(num + 1);
            } else if num > 0 {
                self.tile_type = Blank(num - 1);
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
                    for neighbor in self.get_neighbors(rand_x, rand_y) {
                        if let Some(tile) = self.get_tile_mut(neighbor.0, neighbor.1) {
                            tile.mark_neighbor(true);
                        }
                    }

                    mines -= 1;
                }
            }
        }
    }

    pub fn reveal_tile(&mut self, x: usize, y: usize, try_neighbors: bool) -> bool {
        if let Some(tile) = self.get_tile_mut(x, y) {
            if let Covered = tile.state {
                if let TileType::Mine = tile.tile_type {
                    return true;
                }

                tile.state = Revealed;

                if let Blank(num) = tile.tile_type {
                    if num == 0 {
                        let mut bol = false;

                        for neighbor in self.get_neighbors(x, y) {
                            if let Some(tile2) = self.get_tile_mut(neighbor.0, neighbor.1) {
                                if let Blank(num) = tile2.tile_type {
                                    if let Covered = tile2.state {
                                        if num == 0 {
                                            if self.reveal_tile(neighbor.0, neighbor.1, true) {
                                                bol = true;
                                            }
                                        } else {
                                            tile2.state = Revealed;
                                        }
                                    }
                                }
                            }
                        }

                        return bol;
                    }
                }

                return false;
            } else if let Revealed = tile.state {
                let mut bol = false;

                if try_neighbors {
                    for neighbor in self.get_neighbors(x, y) {
                        if self.reveal_tile(neighbor.0, neighbor.1, false) {
                            bol = true;
                        }
                    }
                }


                return bol;
            }
        }

        return false;
    }

    pub fn flag_tile(&mut self, x: usize, y: usize) -> (i8, bool) {
        let mine = self.is_mine(x, y);

        if let Some(tile) = self.get_tile_mut(x, y) {
            match tile.state {
                Revealed => (0, mine),
                State::Flag => {
                    tile.state = State::Covered;
                    (-1, mine)
                }
                State::Covered => {
                    tile.state = State::Flag;
                    (1, mine)
                }
            }
        } else {
            (0, mine)
        }
    }

    pub fn revealed(&self, x: usize, y: usize) -> bool {
        if let Some(tile) = self.get_tile(x, y) {
            if let Revealed = tile.state {
                return true;
            }
        }

        false
    }

    pub fn flagged(&self, x: usize, y: usize) -> bool {
        if let Some(tile) = self.get_tile(x, y) {
            if let State::Flag = tile.state {
                return true;
            }
        }

        false
    }

    pub fn get_neighbors(&self, x: usize, y: usize) -> Vec<(usize, usize)> {
        let mut neighbors = vec![
            (x + 1, y),
            (x, y + 1),
            (x + 1, y + 1)
        ];

        if x != 0 && y != 0 {
            neighbors.push((x - 1, y - 1));
        }

        if x != 0 {
            neighbors.push((x - 1, y));
            neighbors.push((x - 1, y + 1));
        }

        if y != 0 {
            neighbors.push((x, y - 1));
            neighbors.push((x + 1, y - 1));
        }

        neighbors
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

    let mut mines_left = board.config.mines.clone() as i8;
    let mut real_mines_left = mines_left.clone();
    let mut lost = false;
    let mut win = false;

    let mut rand_instance = RNG::<WyRand, u32>::new(cur_time);

    let mut selected = (0usize, 0usize);

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

        let keys_down = hid.keys_down();

        if keys_down.contains(KeyPad::START) {
            break;
        }

        if keys_down.contains(KeyPad::DPAD_DOWN) || keys_down.contains(KeyPad::CPAD_DOWN) {
            if selected.0 > 0 {
                selected.0 -= 1;
            }
        } else if keys_down.contains(KeyPad::DPAD_UP) || keys_down.contains(KeyPad::CPAD_UP) {
            if selected.0 < (board.config.width - 1) as usize {
                selected.0 += 1;
            }
        } else if keys_down.contains(KeyPad::DPAD_LEFT) || keys_down.contains(KeyPad::CPAD_LEFT) {
            if selected.1 > 0 {
                selected.1 -= 1;
            }
        } else if keys_down.contains(KeyPad::DPAD_RIGHT) || keys_down.contains(KeyPad::CPAD_RIGHT) {
            if selected.1 < (board.config.height - 1) as usize {
                selected.1 += 1;
            }
        } else if !lost && !win {
            if keys_down.contains(KeyPad::A) {
                lost = !board.reveal_tile(selected.0, selected.1, true);
            } else if keys_down.contains(KeyPad::B) {
                let ret = board.flag_tile(selected.0, selected.1);
                mines_left += ret.0;

                if ret.1 {
                    real_mines_left += ret.0;
                }

                if real_mines_left == 0 {
                    win = true;
                }
            }
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

                            if selected.0 == i && selected.1 == j {
                                let color = unsafe {
                                    C2D_Color32f(0.0f32, 0.0f32, 255.0f32, 1.0f32)
                                };

                                unsafe {
                                    C2D_DrawRectangle(
                                        (min_x - pad) as f32,
                                        (min_y - pad) as f32,
                                        0f32,
                                        (x_width + (2*pad)) as f32,
                                        (y_height + (2*pad)) as f32,
                                        color,
                                        color,
                                        color,
                                        color
                                    );
                                }
                            }

                            if let Some(tile) = board.get_tile(i, j) {
                                let mut color: u32 = unsafe {
                                    C2D_Color32f(0.0f32, 100.0f32, 100.0f32, 1.0f32)
                                };

                                if let Revealed = tile.state {
                                    color = unsafe {
                                        C2D_Color32f(255.0f32, 255.0f32, 255.0f32, 1.0f32)
                                    };

                                    if board.is_mine(i, j) {
                                        color = unsafe {
                                            C2D_Color32f(255.0f32, 0.0f32, 0.0f32, 1.0f32)
                                        };
                                    }
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

                                let mut tile_color = tile.tile_type.get_color();

                                if board.flagged(i, j) {
                                    tile_color = unsafe {
                                        C2D_Color32f(255.0f32, 0.0f32, 0.0f32, 1.0f32)
                                    }
                                }

                                if board.revealed(i, j) || board.flagged(i, j) {
                                    unsafe {
                                        C2D_DrawRectangle(
                                            (min_x + pad) as f32,
                                            (min_y + pad) as f32,
                                            0f32,
                                            (x_width - (2*pad)) as f32,
                                            (y_height - (2*pad)) as f32,
                                            tile_color,
                                            tile_color,
                                            tile_color,
                                            tile_color
                                        );
                                    }
                                }
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

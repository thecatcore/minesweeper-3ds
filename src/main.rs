use ctru::prelude::*;
use rand::Rng;

const DEFAULT_CONFIGS: [BoardConfig; 3] = [
    BoardConfig {
        name: "Easy",
        width: 10,
        height: 8,
        mines: 10,
    },
    BoardConfig {
        name: "Medium",
        width: 18,
        height: 14,
        mines: 40,
    },
    BoardConfig {
        name: "Difficult",
        width: 24,
        height: 20,
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

    pub fn place_mines(&mut self) {
        let width = self.config.width;
        let height = self.config.height;
        let mut mines = self.config.mines.clone();

        let mut rand_instance = rand::thread_rng();

        loop {
            if mines == 0 { break }

            let rand_x = rand_instance.gen_range(0..width) as usize;
            let rand_y = rand_instance.gen_range(0..height) as usize;

            if self.is_tile_in_board(rand_x, rand_y) && !self.is_mine(rand_x, rand_y) {
                if let Ok(_) = self.set_tile(rand_x, rand_y, TileType::Mine) {
                    let neighbors = [
                        (rand_x - 1, rand_y - 1),
                        (rand_x, rand_y - 1),
                        (rand_x + 1, rand_y - 1),
                        (rand_x - 1, rand_y),
                        (rand_x + 1, rand_y),
                        (rand_x - 1, rand_y + 1),
                        (rand_x, rand_y + 1),
                        (rand_x + 1, rand_y + 1)
                    ];

                    for neighbor in neighbors {
                        if let Some(tile) = self.get_tile_mut(neighbor.0, neighbor.1) {
                            tile.mark_neighbor(true);
                        }
                    }
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
    let _console = Console::new(gfx.top_screen.borrow_mut());

    let mut board = Board::new(DEFAULT_CONFIGS[0].clone());
    board.place_mines();

    println!("Hello, World!");
    println!("\x1b[29;16HPress Start to exit");

    while apt.main_loop() {
        gfx.wait_for_vblank();

        hid.scan_input();
        if hid.keys_down().contains(KeyPad::START) {
            break;
        }
    }
}

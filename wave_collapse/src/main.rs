use image::{ImageBuffer, GenericImageView, DynamicImage, ImageError};
use std::collections::HashMap;
use rand::Rng;
use glob;
use rand;
use std::fmt;
use ggez::{ContextBuilder, event};
use std::env;
use std::path::Path;
use pbr::ProgressBar;


//use gui::gui::{Gui, Flags};

mod gui;



impl fmt::Display for Grid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for row in &self.cells {
            for cell in row {
                write!(f, "{:?} ", cell.value)?;
            }
            write!(f, "\n")?;
        }
        Ok(())
    }
}

#[derive(Clone)]
struct Tile {
    id: usize,
    name: String,
    value: Option<usize>,
    possible_values: Vec<usize>,
}


impl Tile {
    fn new(id: usize, name: String, possible_values: Vec<usize>) -> Self {
        Self {
            id,
            name,
            value: None,
            possible_values,
        }
    }
}



fn get_ruleset() -> HashMap<usize, Vec<usize>> {
    let mut rules = HashMap::new();
    rules.insert(1, vec![1,2,3]);
    rules.insert(2, vec![1,2]);
    rules.insert(3, vec![1,3,4]);
    rules.insert(4, vec![3,4,5]);
    rules.insert(5, vec![4,5]);
    rules
}

fn stitch_images(grid: &Grid) -> Result<(), ImageError> {
    // Load the first image to get the dimensions
    let first_tile = &grid.cells[0][0];
    let first_tile_name = grid.id_to_name(first_tile.value.unwrap() as u32);
    let first_tile_path = format!("tileset/{}_{}.png", first_tile.value.unwrap(), first_tile_name);
    let first_image: DynamicImage = image::open(&Path::new(&first_tile_path))?;

    // get the dimensions
    let single_image_width = first_image.width();
    let single_image_height = first_image.height();

    // create an empty image with the size of all combined images
    let image_width = single_image_width * grid.cells.len() as u32;
    let image_height = single_image_height * grid.cells.len() as u32;
    let mut final_image = ImageBuffer::new(image_width, image_height);

    for (y, row) in grid.cells.iter().enumerate() {
        for (x, cell) in row.iter().enumerate() {
            if let Some(value) = cell.value {
                let tile_name = grid.id_to_name(value as u32);
                let tile_path = format!("tileset/{}_{}.png", value, tile_name);
                let tile_image = image::open(&Path::new(&tile_path))?.into_rgba8();
                
                // paste the image at the correct position
                let top_left_x = (x * single_image_width as usize) as u32;
                let top_left_y = (y * single_image_height as usize) as u32;
                image::imageops::overlay(&mut final_image, &tile_image, top_left_x, top_left_y);
            }
        }
    }
    

    // save the final image
    final_image.save("final_image.png")
}

fn load_tiles(possible_values: Vec<usize>, current_dir: &Path) -> Vec<Tile> {
    //println!("Current Directory is {}", current_dir as str); 

    let tileset_path = current_dir.join("tileset/*.png");
    let mut tiles = vec![];
    for entry in glob::glob(tileset_path.to_str().unwrap()).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                let filename = path.file_stem().unwrap().to_string_lossy();
                println!("Processing file: {}", filename); // Print the file name being processed

                let parts: Vec<_> = filename.split('_').collect();

                // Check that filename is in expected format
                if parts.len() != 2 {
                    println!("Unexpected filename format: {}", filename);
                    continue;
                }

                let id = match parts[0].parse::<usize>() {
                    Ok(id) => id,
                    Err(_) => {
                        println!("Failed to parse id from filename: {}", filename);
                        continue;
                    }
                };

                let name = parts[1].to_string();
                tiles.push(Tile::new(id, name.clone(), possible_values.clone()));

                println!("Successfully loaded tile with id: {}, name: {}", id, name); 
            }
            Err(e) => println!("Error encountered: {:?}", e), // Print error details
        }
    }
    println!("Loaded {} tiles in total.", tiles.len()); // Print total number of tiles loaded
    tiles
}


pub struct Grid {
    cells: Vec<Vec<Tile>>, // A 2D grid of tiles
    rules: HashMap<usize, Vec<usize>>,
    initial_collapse_done: bool,
}


impl Grid {
    fn new(size: usize, tiles: Vec<Tile>, rules: HashMap<usize, Vec<usize>>) -> Result<Self, &'static str> {
        if tiles.is_empty() {
            return Err("No tiles provided");
        }
        Ok(Self {
            cells: (0..size)
                .map(|_| (0..size)
                .map(|_| {
                    let tile_id = rand::random::<usize>() % tiles.len();
                    let mut tile = tiles[tile_id].clone();
                    tile.value = None; // All tiles start uncollapsed
                    tile
                }).collect())
                .collect(),
            rules,
            initial_collapse_done: false,
        })
    }

    fn entropy(&self, x: usize, y: usize) -> usize {
        if let Some(value) = self.cells[x][y].value {
            if value == 0 { usize::MAX } else { 0 }
        } else {
            self.cells[x][y].possible_values.len()
        }
    }

    fn id_to_name(&self, id: u32) -> &'static str {
        match id {
            1 => "plains",
            2 => "forest",
            3 => "dessert",
            4 => "shore",
            5 => "ocean",
            _ => panic!("Invalid ID"),
        }
    }

    fn collapse(&mut self) -> bool {
        if !self.initial_collapse_done {
            let mid = self.cells.len() / 2;
            println!("Performing Initial collapse at {}, {}", mid, mid);
            if !self.cells[mid][mid].possible_values.is_empty() {
                let idx = rand::thread_rng().gen_range(0..self.cells[mid][mid].possible_values.len());
                self.cells[mid][mid].value = Some(self.cells[mid][mid].possible_values[idx]);
                self.initial_collapse_done = true;
                return true;
            }
        }
        // Find the cell with the smallest non-zero entropy
        let (mut min_x, mut min_y, mut min_entropy) = (0, 0, usize::MAX);
        for (x, row) in self.cells.iter().enumerate() {
            for (y, _cell) in row.iter().enumerate() {
                let entropy = self.entropy(x, y);
                if entropy != 0 && entropy < min_entropy {
                    min_x = x;
                    min_y = y;
                    min_entropy = entropy;
                }
            }
        }
        // Collapse that cell
        if min_entropy != usize::MAX {
            let idx = rand::thread_rng().gen_range(0..self.cells[min_x][min_y].possible_values.len());
            self.cells[min_x][min_y].value = Some(self.cells[min_x][min_y].possible_values[idx]);

            // Remove other possibilities
            self.cells[min_x][min_y].possible_values.clear();
            let val = self.cells[min_x][min_y].value.unwrap();
            self.cells[min_x][min_y].possible_values.push(val);
            // self.cells[min_x][min_y].possible_values.push(self.cells[min_x][min_y].value.unwrap());
            
            return true;
        }
        false
    }
    
    fn propagate(&mut self) {
        let dir = vec![(-1, 0), (1, 0), (0, -1), (0, 1)];  // Up, Down, Left, Right
    
        for i in 0..self.cells.len() {
            for j in 0..self.cells[i].len() {
                if let Some(value) = self.cells[i][j].value {
                    for (dx, dy) in &dir {
                        let nx = i as i32 + dx;
                        let ny = j as i32 + dy;
    
                        if nx >= 0 && nx < self.cells.len() as i32 && ny >= 0 && ny < self.cells[i].len() as i32 {
                            let nx = nx as usize;
                            let ny = ny as usize;
    
                            if let Some(allowed_values) = self.rules.get(&value) {
                                self.cells[nx][ny].possible_values.retain(|v| allowed_values.contains(v));
                            }
                            // Contradiction handling
                            if self.cells[nx][ny].possible_values.is_empty() {
                                println!("Contradiction found at ({}, {})", nx, ny);
                                return;  // You may decide to handle contradictions differently
                            }
                        }
                    }
                }
            }
        }
    }

    // Add function to check for contradictions
    fn has_contradiction(&self) -> bool {
        self.cells.iter().flatten().any(|cell| cell.possible_values.is_empty() && cell.value.is_none())
    }
    
    fn is_fully_collapsed(&self) -> bool {
        // The grid is fully collapsed if every cell has a value
        self.cells.iter().flatten().all(|cell| cell.value.is_some())
    }

    // Modify run function to stop in case of contradictions
    fn run(&mut self) {
        let total_cells = self.cells.len() * self.cells[0].len();
        let mut pb = ProgressBar::new(total_cells as u64);

        while !self.is_fully_collapsed() {
            if self.collapse() {
                pb.inc();
            }
            self.propagate();
            if self.has_contradiction() {
                pb.finish_print("Grid collapsing ended with a contradiction.");
                return;
            }
        }
        pb.finish_print("Grid collapsing completed.");
    }
}

// And then modify the `main` function:

fn main() {
    println!("Initializing Program...");
    let current_dir = env::current_dir().unwrap();
    let possible_values = vec![1, 2, 3, 4, 5]; 
    let tiles = load_tiles(possible_values.clone(), &current_dir);
    let rules = get_ruleset();
    let grid_result = Grid::new(55, tiles, rules);

    let mut grid = match grid_result {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Failed to create grid: {}", e);
            return;
        }
    };

    // let mut game_state = gui::GameState::new(grid);


    // let (ctx, event_loop) = ContextBuilder::new("game_name", "author")
    //     .window_setup(ggez::conf::WindowSetup::default().title("Game Title"))
    //     .window_mode(ggez::conf::WindowMode::default().dimensions(800.0, 600.0))
    //     .build()
    //     .expect("Failed to build ggez context");

    //event::run(ctx, event_loop, game_state);

    grid.run();
    match stitch_images(&grid) {
        Ok(_) => println!("Image stitching completed successfully."),
        Err(e) => println!("Failed to stitch images: {:?}", e),
    }
}


    // match event::run(ctx, event_loop, game_state) {
    //     Ok(_) => println!("Exited cleanly."),
    //     Err(_) => println!("Error occurred during event::run"), // This line is updated
    // }

use std::collections::HashMap;
use rand::Rng;
use glob;
use rand;
use std::fmt;
use std::env;
use image::{ImageBuffer, GenericImageView, DynamicImage, ImageError};
use std::path::Path;

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

fn id_to_name(id: u32) -> &'static str {
    match id {
        1 => "plains",
        2 => "forest",
        3 => "dessert",
        4 => "ocean",
        _ => panic!("Invalid ID"),
    }
}

fn stitch_images(grid: &Grid) -> Result<(), ImageError> {
    // Load the first image to get the dimensions
    let first_tile = &grid.cells[0][0];
    let first_tile_name = id_to_name(first_tile.value.unwrap() as u32);
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
                let tile_name = id_to_name(value as u32);
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


fn get_ruleset() -> HashMap<usize, Vec<usize>> {
    let mut rules = HashMap::new();
    rules.insert(1, vec![1,2,3]);
    rules.insert(2, vec![1,2]);
    rules.insert(3, vec![1,3,4]);
    rules.insert(4, vec![3,4]);
    rules
}

struct Grid {
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
                    tiles[tile_id].clone()
                }).collect())
                .collect(),
            rules,
            initial_collapse_done: false,
        })
    }
    


    fn collapse(&mut self) {
        let mut rng = rand::thread_rng();
        if !self.initial_collapse_done {
            let mid = self.cells.len() / 2;
            println!("Performing Initial collapse at {}, {}", mid, mid);
            if self.cells[mid][mid].value.is_none() && !self.cells[mid][mid].possible_values.is_empty() {
                let idx = rng.gen_range(0..self.cells[mid][mid].possible_values.len());
                self.cells[mid][mid].value = Some(self.cells[mid][mid].possible_values[idx]);
                self.initial_collapse_done = true;
                return;
            }
        }
        'outer: for row in self.cells.iter_mut() {
            for cell in row.iter_mut() {
                if cell.value.is_none() && !cell.possible_values.is_empty() {
                    let idx = rng.gen_range(0..cell.possible_values.len());
                    cell.value = Some(cell.possible_values[idx]);
                    break 'outer;
                }
            }
        }
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
                        }
                    }
                }
            }
        }
    }
    
    fn run(&mut self) {
        // self.collapse();
        // //self.propagate();
        // println!("{}", self); // print the grid after each step
        
        while !self.is_fully_collapsed() {
            self.collapse();
            self.propagate();
            println!("{}", self); // print the grid after each step
        }
    }

    fn is_fully_collapsed(&self) -> bool {
        // The grid is fully collapsed if every cell has a value
        self.cells.iter().flatten().all(|cell| cell.value.is_some())
    }
}

// And then modify the `main` function:
fn main() {
    let current_dir = env::current_dir().unwrap();
    let possible_values = vec![1, 2, 3, 4]; // or whatever you want this to be
    let tiles = load_tiles(possible_values.clone(), &current_dir);
    let rules = get_ruleset();
    let grid_result = Grid::new(30, tiles, rules);

    // Unwrap the grid or exit if there was an error
    let mut grid = match grid_result {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Failed to create grid: {}", e);
            return;
        }
    };

    grid.run();
    match stitch_images(&grid) {
        Ok(_) => println!("Image stitching completed successfully."),
        Err(e) => println!("Failed to stitch images: {:?}", e),
    }
}

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
use std::process;

fn load_image_to_bitmap(image_path: &str) -> Vec<Vec<[i32; 3]>> {
    let img = image::open(image_path).unwrap();
    let (width, height) = img.dimensions();

    let mut bitmap: Vec<Vec<[i32; 3]>> = Vec::new();

    for y in 0..height {
        let mut row: Vec<[i32; 3]> = Vec::new();
        for x in 0..width {
            let pixel = img.get_pixel(x, y);
            let rgb = [pixel[0] as i32, pixel[1] as i32, pixel[2] as i32];  // Ignore the alpha channel
            row.push(rgb);
        }
        bitmap.push(row);
    }

    bitmap
}

fn get_ruleset() -> HashMap<usize, Vec<usize>> {
    let mut rules = HashMap::new();
    rules.insert(1, vec![1,2,3]);
    rules.insert(2, vec![1,2,3]);
    rules.insert(3, vec![1,2,3,4]);
    rules.insert(4, vec![1,3,4,5]);
    rules.insert(5, vec![4,5,6]);
    rules.insert(6, vec![5,6]);
    rules
}

fn stitch_images(grid: &Grid) -> Result<(), ImageError> {
    // Load the first image to get the dimensions
    let first_tile = &grid.cells[0][0];
    let first_tile_name = grid.id_to_name(first_tile.value.unwrap() as u32);

    let first_tile_path = format!("tileset/{}_{}.png", first_tile.value.unwrap(), first_tile_name);

    println!("Trying to open: {}", first_tile_path);

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
    
    println!("Saving final image");
    // save the final image
    final_image.save("final_image.png")
}

fn load_tiles(possible_values: Vec<usize>, current_dir: &Path) -> (Vec<Tile>, HashMap<String, Vec<Vec<Vec<i32>>>>) {
    let tileset_path = current_dir.join("tileset/*.png");
    let mut tiles = vec![];
    let mut tile_transforms: HashMap<String, Vec<Vec<Vec<i32>>>> = HashMap::new();

    for entry in glob::glob(tileset_path.to_str().unwrap()).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                let filename = path.file_stem().unwrap().to_string_lossy();
                println!("Processing file: {}", filename);

                let parts: Vec<_> = filename.split('_').collect();

                if parts.len() != 2 {
                    println!("Unexpected filename format: {}", filename);
                    continue;
                }

                let id = match parts[1].parse::<usize>() {
                    Ok(id) => id,
                    Err(_) => {
                        println!("Failed to parse id from filename: {}", filename);
                        continue;
                    }
                };

                let name = parts[1].to_string();
                let bitmap = vec![];
                
                // Extract symmetry from the filename
                let symmetry = match parts[0].chars().next().unwrap() {
                    'L' => Symmetry::L,
                    'T' => Symmetry::T,
                    'I' => Symmetry::I,
                    '\\' => Symmetry::BackSlash,
                    '/' => Symmetry::ForwardSlash,
                    'F' => Symmetry::F,
                    'X' => Symmetry::X,
                    _ => {
                        println!("Invalid symmetry in filename: {}", filename);
                        continue;
                    }
                };

                let weight = 0.0;
                tiles.push(Tile::new(id, name.clone(), bitmap, symmetry, weight, possible_values.clone()));
                tile_transforms.insert(name.clone(), tiles.last().unwrap().generate_transforms());

                println!("Successfully loaded tile with id: {}, name: {}", id, name);
            }
            Err(e) => println!("Error encountered: {:?}", e),
        }
    }
    println!("Loaded {} tiles in total.", tiles.len());
    (tiles, tile_transforms)
}

#[derive(Clone)]
enum Symmetry {
    L,
    T,
    I,
    BackSlash,
    ForwardSlash,
    F,
    X
}

pub struct Grid {
    cells: Vec<Vec<Tile>>, // A 2D grid of tiles
    rules: HashMap<usize, Vec<usize>>,
    initial_collapse_done: bool,
    tile_transforms: HashMap<String, Vec<Vec<Vec<i32>>>>, // Use String as the key
}

#[derive(Clone)]
struct Tile {
    id: usize,
    name: String,
    symmetry: Symmetry,
    weight: f32,
    value: Option<usize>,
    bitmap: Vec<Vec<i32>>, 
    possible_values: Vec<usize>,
}

impl Tile {
    fn new(id: usize, name: String, bitmap: Vec<Vec<i32>>, symmetry: Symmetry, weight: f32, possible_values: Vec<usize>) -> Self {
        Self {
            id,
            name,
            symmetry,
            weight,
            value: None,
            bitmap,
            possible_values,
        }
    }

    fn rotate_cw_twice(&self) -> Vec<Vec<i32>> {
        let mut temp = self.clone();
        temp.bitmap = self.rotate_cw();
        temp.rotate_cw()
    }

    fn rotate_cw(&self) -> Vec<Vec<i32>> {
        let n = self.bitmap.len();
        let mut new_bitmap = vec![vec![0; n]; n];
        for i in 0..n {
            for j in 0..n {
                new_bitmap[j][n-i-1] = self.bitmap[i][j];
            }
        }
        new_bitmap
    }
    
    fn reflect(&self) -> Vec<Vec<i32>> {
        let n = self.bitmap.len();
        let mut new_bitmap = vec![vec![0; n]; n];
        for i in 0..n {
            for j in 0..n {
                new_bitmap[i][n-j-1] = self.bitmap[i][j];
            }
        }
        new_bitmap
    }

    fn generate_transforms(&self) -> Vec<Vec<Vec<i32>>> {
        let mut transforms = vec![self.bitmap.clone()];
        let cw1 = self.rotate_cw();
        let cw2 = self.clone().rotate_cw();  // self.clone() to get a new Tile instance
        let cw3 = self.clone().rotate_cw_twice();
        let refl = self.reflect();
    
        match self.symmetry {
            Symmetry::L => {
                transforms.extend_from_slice(&[cw1, cw2, cw3]);
            },
            Symmetry::T => {
                transforms.extend_from_slice(&[cw1, cw2]);
            },
            Symmetry::I => {
                transforms.push(cw1);
            },
            Symmetry::BackSlash => {
                transforms.push(refl);
            },
            Symmetry::F => {
                transforms.extend_from_slice(&[refl, cw1, cw2, cw3]);
            },
            Symmetry::X => {
                transforms.push(refl);
            },
        }
        transforms
    }
    
}

impl Grid {
    fn new(size: usize, tiles: Vec<Tile>, rules: HashMap<usize, Vec<usize>>, tile_transforms: HashMap<String, Vec<Vec<Vec<i32>>>>) -> Result<Self, &'static str> {
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
            tile_transforms,
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
            3 => "mountains",
            4 => "dessert",
            5 => "shore",
            6 => "ocean",
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
    
    fn propagate(&mut self) -> Result<(), ()> {
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
    
                            // Get all transformations of the neighboring tile

                            if let Some(neighbor_value) = self.cells[nx][ny].value {
                                if let Some(neighbor_transforms) = self.tile_transforms.get(&neighbor_value.to_string()) {
                                    let neighbor_transforms = self.tile_transforms[&self.cells[nx][ny].value.unwrap().to_string()].clone();
                            
                                    // Check each possible value of the current cell against all transformations of the neighbor
                                    self.cells[i][j].possible_values.retain(|v| {
                                        let allowed_values = self.rules.get(v).unwrap();
                                    
                                        neighbor_transforms.iter().flat_map(|x| x.iter().flat_map(|y| y.iter())).any(|t| {
                                            let t_usize = *t as usize; // Convert i32 to usize
                                            allowed_values.contains(&t_usize)
                                        })
                                    });
                                } else {
                                    //println!("No transformations found for the neighboring cell at ({}, {})", nx, ny);
                                }
                            } else {
                                //println!("No value found for the neighboring cell at ({}, {})", nx, ny);
                            }


    
                            // Contradiction handling
                            if self.cells[i][j].possible_values.is_empty() {
                                println!("Contradiction found at ({}, {})", i, j);
                                return Err(());
                            }
                        }
                    }
                }
            }
        }
        Ok(())
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
            if let Err(()) = self.propagate() {  // Handle error from propagate
                pb.finish_print("Grid collapsing ended with a contradiction.");
                return;
            }
        }
        pb.finish_print("Grid collapsing completed.");
    }
}

fn main() {
    println!("Initializing Program...");
    let current_dir = env::current_dir().unwrap();
    let possible_values = vec![1, 2, 3, 4, 5, 6]; 
    let (tiles, tile_transforms) = load_tiles(possible_values.clone(), &current_dir);
    let rules = get_ruleset();
    let grid_result = Grid::new(85, tiles, rules, tile_transforms);

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



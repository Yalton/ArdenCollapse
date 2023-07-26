use ggez::{event, graphics, Context, GameResult, graphics::Image};
use pbr::ProgressBar;
use crate::Grid;
use crate::ImageBuffer;
use image::{DynamicImage, GenericImageView};
use std::path::Path;

// Create GameState struct
pub struct GameState {
    grid: Grid,
    final_image: Option<ggez::graphics::Image>,
}

impl GameState {
    pub fn new(passed_grid: Grid) -> Self {
        GameState { 
            grid : passed_grid,
            final_image: None,
        }
    }
}

impl event::EventHandler<ggez::GameError> for GameState {
    fn update(&mut self, ctx: &mut ggez::Context) -> ggez::GameResult<()> {
        println!("Entering GUI::Update");
        let total_cells = self.grid.cells.len() * self.grid.cells[0].len();
        let mut pb = ProgressBar::new(total_cells as u64);
        
        let first_tile_path = format!("tileset/{}_{}.png", 1, "plains");

        println!("Trying to open: {}", first_tile_path);
    
        let first_image: DynamicImage = image::open(&Path::new(&first_tile_path))?;
    
        
        let mut image_width = first_image.width();
        let mut image_height = first_image.height();
        
        // get the dimensions
        let mut single_image_width = first_image.width();
        let mut single_image_height = first_image.height();
    
        let image_width = single_image_width * self.grid.cells.len() as u32;
        let image_height = single_image_height * self.grid.cells.len() as u32;
                            
        let mut final_image_buffer = ImageBuffer::new(image_width, image_height);
            
        while !self.grid.is_fully_collapsed() {
            if self.grid.collapse() {
                pb.inc();
                for (y, row) in self.grid.cells.iter().enumerate() {
                    for (x, cell) in row.iter().enumerate() {
                        if let Some(value) = cell.value {
                            let tile_name = self.grid.id_to_name(value as u32);
                            let tile_path = format!("tileset/{}_{}.png", value, tile_name);
                            if Path::new(&tile_path).exists() {
                                let tile_image = image::open(&Path::new(&tile_path))?.into_rgba8();
                                                
                                // Update single_image dimensions
                                single_image_width = tile_image.width();
                                single_image_height = tile_image.height();
                        
                                let image_width = single_image_width * self.grid.cells.len() as u32;
                                let image_height = single_image_height * self.grid.cells.len() as u32;
                                                    
                                // if final_image_buffer.width() == 0 && final_image_buffer.height() == 0 {
                                //     final_image_buffer = ImageBuffer::new(image_width, image_height);
                                // }
                            
                                let top_left_x = (x * single_image_width as usize) as u32;
                                let top_left_y = (y * single_image_height as usize) as u32;
                                image::imageops::overlay(&mut final_image_buffer, &tile_image, top_left_x, top_left_y);
                            }
                        }
                    }
                }
    
                // Convert your image::DynamicImage or image::ImageBuffer to ggez::graphics::Image
                let image_data = final_image_buffer.clone().into_raw();
                let ggez_image = Image::from_rgba8(ctx, image_width as u16, image_height as u16, &image_data)?;
    
                // Update the final_image in GameState
                self.final_image = Some(ggez_image);
            }
            self.grid.propagate();
            if self.grid.has_contradiction() {
                pb.finish_print("Grid collapsing ended with a contradiction.");
                event::quit(ctx);
                return Ok(());
            }
        }
        pb.finish_print("Grid collapsing completed.");
        Ok(())
    }

    fn draw(&mut self, ctx: &mut ggez::Context) -> ggez::GameResult {
        graphics::clear(ctx, [0.1, 0.2, 0.3, 1.0].into());
        
        if let Some(image) = &self.final_image {
            let draw_params = ggez::graphics::DrawParam::default();
            graphics::draw(ctx, image, draw_params)?;
        }
    
        graphics::present(ctx)?;
        Ok(())
    }
    
}

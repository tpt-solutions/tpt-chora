pub struct OverdrawHeatmap {
    width: u32,
    height: u32,
    cells: Vec<u32>,
    cell_size: u32,
    max_overdraw: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct HeatmapCell {
    pub x: u32,
    pub y: u32,
    pub overdraw_count: u32,
    pub intensity: f32,
}

impl OverdrawHeatmap {
    pub fn new(width: u32, height: u32) -> Self {
        let cell_size = 8;
        let cells_x = width.div_ceil(cell_size);
        let cells_y = height.div_ceil(cell_size);

        Self {
            width,
            height,
            cells: vec![0; (cells_x * cells_y) as usize],
            cell_size,
            max_overdraw: 1,
        }
    }

    pub fn record_triangle(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, x2: f32, y2: f32) {
        let min_x = x0.min(x1).min(x2) as u32;
        let max_x = (x0.max(x1).max(x2) as u32).min(self.width - 1);
        let min_y = y0.min(y1).min(y2) as u32;
        let max_y = (y0.max(y1).max(y2) as u32).min(self.height - 1);

        let cells_x = self.width.div_ceil(self.cell_size);

        for y in (min_y..=max_y).step_by(self.cell_size as usize) {
            for x in (min_x..=max_x).step_by(self.cell_size as usize) {
                let cx = x / self.cell_size;
                let cy = y / self.cell_size;
                let idx = (cy * cells_x + cx) as usize;
                if idx < self.cells.len() {
                    self.cells[idx] += 1;
                    self.max_overdraw = self.max_overdraw.max(self.cells[idx]);
                }
            }
        }
    }

    pub fn reset(&mut self) {
        self.cells.fill(0);
        self.max_overdraw = 1;
    }

    pub fn get_cells(&self) -> Vec<HeatmapCell> {
        let cells_x = self.width.div_ceil(self.cell_size);
        self.cells
            .iter()
            .enumerate()
            .filter(|&(_, &count)| count > 0)
            .map(|(i, &count)| {
                let x = (i as u32 % cells_x) * self.cell_size;
                let y = (i as u32 / cells_x) * self.cell_size;
                HeatmapCell {
                    x,
                    y,
                    overdraw_count: count,
                    intensity: count as f32 / self.max_overdraw as f32,
                }
            })
            .collect()
    }

    pub fn to_rgba_texture_data(&self) -> Vec<u8> {
        let cells_x = self.width.div_ceil(self.cell_size);
        let _cells_y = self.height.div_ceil(self.cell_size);
        let mut data = vec![0u8; (self.width * self.height * 4) as usize];

        for (i, &count) in self.cells.iter().enumerate() {
            let cx = i as u32 % cells_x;
            let cy = i as u32 / cells_x;
            let intensity = count as f32 / self.max_overdraw as f32;

            let r = (intensity * 255.0) as u8;
            let g = ((1.0 - intensity) * 128.0) as u8;
            let b = ((1.0 - intensity) * 255.0) as u8;
            let a = if count > 0 { 180 } else { 0 };

            for dy in 0..self.cell_size {
                for dx in 0..self.cell_size {
                    let px = cx * self.cell_size + dx;
                    let py = cy * self.cell_size + dy;
                    if px < self.width && py < self.height {
                        let idx = ((py * self.width + px) * 4) as usize;
                        if idx + 3 < data.len() {
                            data[idx] = r;
                            data[idx + 1] = g;
                            data[idx + 2] = b;
                            data[idx + 3] = a;
                        }
                    }
                }
            }
        }

        data
    }
}

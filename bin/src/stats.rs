#[derive(Debug)]
pub(crate) struct ProcessingStats {
    pub(crate) total_files: usize,
    pub(crate) processed: usize,
    pub(crate) failed: usize,
    pub(crate) total_polygons: usize,
}

impl ProcessingStats {
    pub(crate) fn new(total_files: usize) -> Self {
        Self {
            total_files,
            processed: 0,
            failed: 0,
            total_polygons: 0,
        }
    }

    pub(crate) fn print_progress(&self) {
        println!(
            "Progress: {}/{} files processed, {} failed, {} polygons generated",
            self.processed + self.failed,
            self.total_files,
            self.failed,
            self.total_polygons
        );
    }

    pub(crate) fn print_summary(&self) {
        println!("\n=== Processing Summary ===");
        println!("Total files: {}", self.total_files);
        println!("Successfully processed: {}", self.processed);
        println!("Failed: {}", self.failed);
        println!("Total polygons generated: {}", self.total_polygons);
        println!("Success rate: {:.1}%",
                 (self.processed as f64 / self.total_files as f64) * 100.0);
    }
}
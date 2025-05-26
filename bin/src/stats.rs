use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub(crate) struct Benchmark {
    initial_instant: Instant,
    instant: Instant,
    steps: Vec<StepBenchmark>
}

impl Benchmark {
    
    pub fn now() -> Self {
        Self {
            initial_instant: Instant::now(),
            instant: Instant::now(),
            steps: Vec::new(),
        }
    }
    
    pub fn step(&mut self, name: &str) {
        self.steps.push(StepBenchmark {
            name: name.to_string(),
            duration: self.instant.elapsed(),
        });
        self.instant = Instant::now();
    }
    
    pub fn get_steps(&self) -> &Vec<StepBenchmark> {
        self.steps.as_ref()
    }
    
    pub fn get_total_duration(&self) -> Duration {
        self.instant.duration_since(self.initial_instant)
    }
    
}

#[derive(Debug, Clone)]
pub(crate) struct StepBenchmark {
    pub(crate) name: String,
    pub(crate) duration: Duration,
}

#[derive(Debug, Clone)]
pub(crate) struct MeshStats {
    pub(crate) vertex_count_2d: usize,
    pub(crate) triangle_count_2d: usize,
    pub(crate) vertex_count_3d: usize,
    pub(crate) triangle_count_3d: usize,
}

#[derive(Debug)]
pub(crate) struct ProcessingResult {
    pub(crate) polygon_count: usize,
    pub(crate) mesh_stats: Vec<MeshStats>,
    pub(crate) benchmarks: Benchmark,
    pub(crate) total_duration: Duration,
}

#[derive(Debug)]
pub(crate) struct ProcessingStats {
    pub(crate) total_files: usize,
    pub(crate) processed: usize,
    pub(crate) failed: usize,
    pub(crate) total_polygons: usize,
    pub(crate) total_vertices_2d: usize,
    pub(crate) total_vertices_3d: usize,
    pub(crate) total_triangles_2d: usize,
    pub(crate) total_triangles_3d: usize,
    pub(crate) total_processing_time: Duration,
    pub(crate) benchmarks_summary: Vec<(String, Duration, usize)>, // name, total_time, count
}

impl ProcessingStats {
    pub(crate) fn new(total_files: usize) -> Self {
        Self {
            total_files,
            processed: 0,
            failed: 0,
            total_polygons: 0,
            total_vertices_2d: 0,
            total_vertices_3d: 0,
            total_triangles_2d: 0,
            total_triangles_3d: 0,
            total_processing_time: Duration::new(0, 0),
            benchmarks_summary: Vec::new(),
        }
    }

    pub(crate) fn add_result(&mut self, result: ProcessingResult) {
        self.processed += 1;
        self.total_polygons += result.polygon_count;
        self.total_processing_time += result.total_duration;

        for mesh_stat in &result.mesh_stats {
            self.total_vertices_2d += mesh_stat.vertex_count_2d;
            self.total_vertices_3d += mesh_stat.vertex_count_3d;
            self.total_triangles_2d += mesh_stat.triangle_count_2d;
            self.total_triangles_3d += mesh_stat.triangle_count_3d;
        }

        // Aggregate benchmark data
        for benchmark in result.benchmarks.get_steps() {
            if let Some(summary) = self.benchmarks_summary.iter_mut()
                .find(|(name, _, _)| name == &benchmark.name) {
                summary.1 += benchmark.duration;
                summary.2 += 1;
            } else {
                self.benchmarks_summary.push((benchmark.name.clone(), benchmark.duration, 1));
            }
        }
    }

    pub(crate) fn add_failure(&mut self) {
        self.failed += 1;
    }

    pub(crate) fn print_progress(&self) {
        println!(
            "Progress: {}/{} files processed, {} failed, {} polygons, {} total vertices (3D)",
            self.processed + self.failed,
            self.total_files,
            self.failed,
            self.total_polygons,
            self.total_vertices_3d
        );
    }

    pub(crate) fn print_summary(&self) {
        println!("\n=== Processing Summary ===");
        println!("Total files: {}", self.total_files);
        println!("Successfully processed: {}", self.processed);
        println!("Failed: {}", self.failed);
        println!("Success rate: {:.1}%",
                 (self.processed as f64 / self.total_files as f64) * 100.0);

        println!("\n=== Geometry Statistics ===");
        println!("Total polygons generated: {}", self.total_polygons);
        println!("Total 2D vertices: {}", self.total_vertices_2d);
        println!("Total 3D vertices: {}", self.total_vertices_3d);
        println!("Total 2D triangles: {}", self.total_triangles_2d);
        println!("Total 3D triangles: {}", self.total_triangles_3d);

        println!("\n=== Performance Summary ===");
        println!("Total processing time: {:.2}s", self.total_processing_time.as_secs_f64());

        println!("\n=== Step Performance ===");
        for (name, total_time, count) in &self.benchmarks_summary {
            println!("{}: {:.2}s total, {:.2}s avg ({} files)",
                     name,
                     total_time.as_secs_f64(),
                     total_time.as_secs_f64() / *count as f64,
                     count);
        }
    }
}
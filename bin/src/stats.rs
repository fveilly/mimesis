use std::path::PathBuf;
use std::sync::Mutex;
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

#[derive(Debug, Clone)]
pub(crate) struct ProcessingResult {
    pub(crate) input: PathBuf,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) polygon_count: usize,
    pub(crate) mesh_stats: Vec<MeshStats>,
    pub(crate) benchmarks: Benchmark,
    pub(crate) total_duration: Duration,
}

static PRINT_LOCK: Mutex<()> = Mutex::new(());

impl ProcessingResult {
    pub fn print_success_compact(&self) {
        let _lock = PRINT_LOCK.lock().unwrap();

        let total_2d_vertices: usize = self.mesh_stats.iter().map(|s| s.vertex_count_2d).sum();
        let total_3d_vertices: usize = self.mesh_stats.iter().map(|s| s.vertex_count_3d).sum();

        println!("✓ {} | {}x{} | {} polygons | {}/{} vertices | {:.1}ms",
                 self.input.file_name().unwrap_or_default().to_string_lossy(),
                 self.width,
                 self.height,
                 self.polygon_count,
                 total_2d_vertices,
                 total_3d_vertices,
                 self.total_duration.as_millis()
        );
    }

    pub fn print_success_detailed(&self, show_benchmarks: bool, show_mesh_details: bool) {
        let _lock = PRINT_LOCK.lock().unwrap();

        println!("{}", "─".repeat(80));
        println!("✓ PROCESSING COMPLETE");
        println!("  File: {}", self.input.display());
        println!("  Image: {}×{} pixels", self.width, self.height);
        println!("  Polygons: {}", self.polygon_count);
        println!("  Total Time: {:.3}s", self.total_duration.as_secs_f64());

        if show_mesh_details && !self.mesh_stats.is_empty() {
            println!("\n  📊 MESH STATISTICS:");
            let mut total_2d_verts = 0;
            let mut total_3d_verts = 0;
            let mut total_2d_tris = 0;
            let mut total_3d_tris = 0;

            for (i, stats) in self.mesh_stats.iter().enumerate() {
                println!("    Mesh {:2}: 2D ({:>6}v, {:>6}t) | 3D ({:>6}v, {:>6}t)",
                         i + 1,
                         stats.vertex_count_2d,
                         stats.triangle_count_2d,
                         stats.vertex_count_3d,
                         stats.triangle_count_3d
                );

                total_2d_verts += stats.vertex_count_2d;
                total_3d_verts += stats.vertex_count_3d;
                total_2d_tris += stats.triangle_count_2d;
                total_3d_tris += stats.triangle_count_3d;
            }

            println!("    Total:    2D ({:>6}v, {:>6}t) | 3D ({:>6}v, {:>6}t)",
                     total_2d_verts, total_2d_tris, total_3d_verts, total_3d_tris);
        }

        if show_benchmarks && !self.benchmarks.get_steps().is_empty() {
            println!("\n  ⏱️  TIMING BREAKDOWN:");
            for step in self.benchmarks.get_steps() {
                let percentage = (step.duration.as_millis() as f64 / self.total_duration.as_millis() as f64) * 100.0;
                println!("    {:.<25} {:>8.2}ms ({:>5.1}%)",
                         step.name,
                         step.duration.as_millis(),
                         percentage
                );
            }
        }

        println!("{}", "─".repeat(80));
    }
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

fn format_number(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs_f64();

    if total_secs >= 3600.0 {
        format!("{:.1}h", total_secs / 3600.0)
    } else if total_secs >= 60.0 {
        format!("{:.1}m", total_secs / 60.0)
    } else if total_secs >= 1.0 {
        format!("{:.2}s", total_secs)
    } else {
        format!("{}ms", duration.as_millis())
    }
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len-3])
    }
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
        let _lock = PRINT_LOCK.lock().unwrap();

        let processed_total = self.processed + self.failed;
        let success_rate = if processed_total > 0 {
            (self.processed as f64 / processed_total as f64) * 100.0
        } else {
            0.0
        };

        // Progress bar visualization
        let progress_width = 30;
        let progress_ratio = processed_total as f64 / self.total_files as f64;
        let filled_chars = (progress_ratio * progress_width as f64) as usize;
        let progress_bar = "█".repeat(filled_chars) + &"░".repeat(progress_width - filled_chars);

        println!(
            "📊 Progress: [{progress_bar}] {processed_total:>3}/{total} ({progress:.1}%) | ✓{success} ✗{failed} | {polygons:>6} polygons | {vertices:>8} vertices",
            progress_bar = progress_bar,
            processed_total = processed_total,
            total = self.total_files,
            progress = progress_ratio * 100.0,
            success = self.processed,
            failed = self.failed,
            polygons = format_number(self.total_polygons),
            vertices = format_number(self.total_vertices_3d)
        );

        if self.failed > 0 {
            println!("   ⚠️  Success rate: {:.1}%", success_rate);
        }
    }
    
    pub(crate) fn print_summary(&self, show_benchmarks: bool, show_mesh_details: bool) {
        let _lock = PRINT_LOCK.lock().unwrap();

        println!("\n{}", "─".repeat(80));
        println!("{:^80}", "🎯 PROCESSING SUMMARY");
        println!("{}", "─".repeat(80));

        let success_rate = if self.total_files > 0 {
            (self.processed as f64 / self.total_files as f64) * 100.0
        } else {
            0.0
        };

        // File processing statistics
        println!("📁 Files:");
        println!("   Total files:        {:>8}", format_number(self.total_files));
        println!("   Successfully processed: {:>4} ({:.1}%)",
                 format_number(self.processed), success_rate);

        if self.failed > 0 {
            println!("   Failed:             {:>4} ({:.1}%)",
                     format_number(self.failed),
                     (self.failed as f64 / self.total_files as f64) * 100.0);
        }

        if show_mesh_details {
            println!("\n📐 Geometry Statistics:");
            println!("   Polygons generated:     {:>8}", format_number(self.total_polygons));
            println!("   2D vertices:            {:>8}", format_number(self.total_vertices_2d));
            println!("   3D vertices:            {:>8}", format_number(self.total_vertices_3d));
            println!("   2D triangles:           {:>8}", format_number(self.total_triangles_2d));
            println!("   3D triangles:           {:>8}", format_number(self.total_triangles_3d));
        }

        if show_benchmarks {
            println!("\n⏱️  Performance:");
            println!("   Total processing time:  {:>8}", format_duration(self.total_processing_time));

            if self.processed > 0 {
                let avg_time = self.total_processing_time.as_secs_f64() / self.processed as f64;
                println!("   Average time per file:  {:>8.2}s", avg_time);
            }
        }

        println!()
    }
    
    pub(crate) fn print_summary_full(&self, show_benchmarks: bool, show_mesh_details: bool) {
        self.print_summary(show_benchmarks, show_mesh_details);

        if show_benchmarks && !self.benchmarks_summary.is_empty() {
            let _lock = PRINT_LOCK.lock().unwrap();

            println!("\n{}", "─".repeat(80));
            println!("{:^80}", "📈 DETAILED PERFORMANCE BREAKDOWN");
            println!("{}", "─".repeat(80));

            // Header
            println!("{:<25} {:>12} {:>12} {:>12} {:>8}",
                     "Step Name", "Total Time", "Avg Time", "Per File", "Files");
            println!("{}", "─".repeat(80));

            // Sort by total time descending
            let mut sorted_benchmarks = self.benchmarks_summary.clone();
            sorted_benchmarks.sort_by(|a, b| b.1.cmp(&a.1));

            let total_time = self.total_processing_time.as_secs_f64();

            for (name, total_step_time, count) in &sorted_benchmarks {
                let avg_time = total_step_time.as_secs_f64() / *count as f64;
                let percentage = (total_step_time.as_secs_f64() / total_time) * 100.0;

                println!("{:<25} {:>12} {:>12.3}s {:>11.1}% {:>8}",
                         truncate_string(name, 25),
                         format_duration(*total_step_time),
                         avg_time,
                         percentage,
                         format_number(*count));
            }

            println!("{}", "─".repeat(80));

            // Performance insights
            if let Some((slowest_step, slowest_time, _)) = sorted_benchmarks.first() {
                println!("🔍 Insights:");
                println!("   Slowest step: {} ({:.1}% of total time)",
                         slowest_step,
                         (slowest_time.as_secs_f64() / total_time) * 100.0);

                if sorted_benchmarks.len() > 1 {
                    let (fastest_step, fastest_time, _) = &sorted_benchmarks[sorted_benchmarks.len() - 1];
                    let speed_ratio = slowest_time.as_secs_f64() / fastest_time.as_secs_f64();
                    println!("   Speed difference: {:.1}x between fastest ({}) and slowest step",
                             speed_ratio, fastest_step);
                }
            }

            println!("{}", "─".repeat(80));
        }
    }
    
    pub(crate) fn print_status_line(&self) {
        let _lock = PRINT_LOCK.lock().unwrap();

        let processed_total = self.processed + self.failed;
        let progress = if self.total_files > 0 {
            (processed_total as f64 / self.total_files as f64) * 100.0
        } else {
            0.0
        };

        print!("\r🔄 [{:>3.0}%] {}/{} files | ✓{} ✗{} | {:.1}s elapsed",
               progress,
               processed_total,
               self.total_files,
               self.processed,
               self.failed,
               self.total_processing_time.as_secs_f64());

        use std::io::{self, Write};
        io::stdout().flush().unwrap();
    }
}
mod error;

use std::{
    collections::VecDeque,
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
    os::unix::process::ExitStatusExt,
    time::{Duration, Instant},
};

use memmap2::MmapOptions;
use rand::Rng;
use log::{info, warn, error, debug};

use crate::{
    cli::Args,
    coverage::{CoverageMetric, create_coverage_metric},
};
pub use error::{FuzzerError, Result};

const COVERAGE_SHM_PATH: &str = "/tmp/coverage_shm.bin";
const COVERAGE_SHM_SIZE: usize = 512 * 1024 * 1024; // 512MB

/// Test case representation
#[derive(Clone)]
struct TestCase {
    /// Name of the file in the queue directory
    filename: String,
}

/// Statistics tracking for the fuzzing session
#[derive(Default)]
struct Stats {
    total_executions: usize,
    new_coverage_count: usize,
    crash_count: usize,
    start_time: Option<Instant>,
    last_status_time: Option<Instant>,
    level: usize,
}

impl Stats {
    fn new() -> Self {
        Self {
            start_time: Some(Instant::now()),
            ..Default::default()
        }
    }

    fn should_update_status(&self) -> bool {
        self.last_status_time
            .map(|t| t.elapsed() >= Duration::from_secs(1))
            .unwrap_or(true)
    }
}

pub struct Fuzzer {
    args: Args,
    queue: VecDeque<TestCase>,
    coverage: Box<dyn CoverageMetric>,
    uses_file_input: bool,
    queue_dir: PathBuf,      // Directory for storing queue files
    crashes_dir: PathBuf,    // Directory for storing crashes
    next_id: usize,          // Counter for generating unique IDs
    stats: Stats,            // Statistics tracking
    coverage_mmap: memmap2::MmapMut,  // Add this field
}

impl Fuzzer {
    fn create_coverage_shm() -> Result<memmap2::MmapMut> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(COVERAGE_SHM_PATH)?;
        
        file.set_len(COVERAGE_SHM_SIZE as u64)?;
        
        let mut mmap = unsafe { MmapOptions::new().map_mut(&file)? };
        mmap.fill(0);
        
        Ok(mmap)
    }

    pub fn new(args: Args) -> Result<Self> {
        let input_marker_count = args.target_cmd.iter()
            .filter(|arg| arg.to_str().map_or(false, |s| s == "@@"))
            .count();
        if input_marker_count > 1 {
            return Err(FuzzerError::Configuration(
                "Multiple @@ markers found in command line. Only one marker is supported.".to_string()
            ));
        }
        let uses_file_input = input_marker_count > 0;

        // Create output directory structure
        let queue_dir = args.output_dir.join("queue");
        let crashes_dir = args.output_dir.join("crashes");
        fs::create_dir_all(&queue_dir)?;
        fs::create_dir_all(&crashes_dir)?;

        // Create and initialize shared memory
        info!("Creating shared memory of size {} MB...", COVERAGE_SHM_SIZE / 1024 / 1024);
        let coverage_mmap = Self::create_coverage_shm()?;

        Ok(Self {
            coverage: create_coverage_metric(args.coverage_type),
            args,
            queue: VecDeque::new(),
            uses_file_input,
            queue_dir,
            crashes_dir,
            next_id: 0,
            stats: Stats::new(),
            coverage_mmap,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        self.load_initial_seeds()?;
        self.fuzz_loop()
    }

    fn get_queue_path(&self, filename: &str) -> PathBuf {
        self.queue_dir.join(filename)
    }

    fn get_crash_path(&self, filename: &str) -> PathBuf {
        self.crashes_dir.join(filename)
    }

    fn save_to_queue(&mut self, data: &[u8], new_coverage: bool) -> Result<String> {
        let filename = if new_coverage {
            format!("id:{:06}:+cov", self.next_id)
        } else {
            format!("id:{:06}", self.next_id)
        };
        self.next_id += 1;

        let path = self.get_queue_path(&filename);
        debug!("Saving to queue: {}", path.display());
        fs::write(path, data)?;

        Ok(filename)
    }

    fn save_crash(&mut self, data: &[u8], signal: i32) -> Result<()> {
        let filename = format!("crash:{:06},sig:{}", self.next_id, signal);
        let path = self.get_crash_path(&filename);
        fs::write(path, data)?;
        self.stats.crash_count += 1;
        Ok(())
    }

    /// Runs the target program with the given input and collects coverage information
    /// 
    /// # Arguments
    /// * `input` - The input data to feed to the target program
    ///
    /// # Returns
    /// * `Ok((Vec<u32>, bool))` containing:
    ///   - The execution path (block IDs) taken during execution
    ///   - Whether this input triggered new coverage
    /// * `Err` if there was an error running the target or collecting coverage
    fn run_and_get_coverage(&mut self, input: &[u8]) -> Result<(Vec<u32>, bool)> {
        self.stats.total_executions += 1;
        if self.stats.should_update_status() {
            self.print_status();
            self.stats.last_status_time = Some(Instant::now());
        }

        // Prepare command
        let mut cmd = Command::new(&self.args.target_cmd[0]);
        
        // Create temp file outside the loop if we need it
        let temp_file = if self.uses_file_input {
            let mut temp = tempfile::NamedTempFile::new()?;
            temp.write_all(input)?;
            // println!("Created temp file at: {:?}", temp.path());
            Some(temp)
        } else {
            None
        };
        
        // Add arguments, replacing @@ with temp file path if needed
        for arg in &self.args.target_cmd[1..] {
            if arg == "@@" {
                if let Some(temp) = &temp_file {
                    cmd.arg(temp.path());
                }
            } else {
                cmd.arg(arg);
            }
        }

        // Configure stdio
        if !self.uses_file_input {
            cmd.stdin(Stdio::piped());
        }
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::piped());

        // Set RUST_BACKTRACE environment variable
        cmd.env("RUST_BACKTRACE", "1");

        // Use the stored mapping instead of creating a new one
        if self.coverage_mmap.len() >= 4 {
            self.coverage_mmap[0..4].copy_from_slice(&0u32.to_ne_bytes());
        } else {
            error!("Coverage file is too short to clear execution path");
        }

        // Run the target
        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                error!("Failed to spawn process: {}", e);
                return Err(e.into());
            }
        };
        
        // Write input to stdin if not using file input
        if !self.uses_file_input {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(input)?;
            }
        }

        // Wait for completion
        let status = child.wait()?;
        let mut _crashed = false;
        if !status.success() {
            // Check if process was terminated by a signal (crash)
            if let Some(signal) = status.signal() {
                // SIGSEGV = 11, SIGABRT = 6, SIGBUS = 7
                match signal {
                    11 | 6 | 7 => {
                        _crashed = true;
                        // Save crash
                        self.save_crash(input, signal)?;
                    },
                    _ => {
                        warn!("Target terminated by unhandled signal: {}", signal);
                        // Continue fuzzing
                    }
                }
            }
        }

        let mut path = Vec::new();
        if self.coverage_mmap.len() >= 4 {
            let len = u32::from_ne_bytes(self.coverage_mmap[0..4].try_into().unwrap()) as usize;
            debug!("Coverage path length: {}", len);
            for i in 0..len {
                let offset = 4 + i * 4;
                let block_id = u32::from_ne_bytes(
                    self.coverage_mmap[offset..offset + 4].try_into().unwrap()
                );
                path.push(block_id);
            }
            debug!("Coverage path: {:?}", path);
        }

        // Check if this path triggers new coverage
        let trigger_new_cov = self.coverage.update_from_path(&path);
        if trigger_new_cov {
            self.stats.new_coverage_count += 1;
        }

        Ok((path, trigger_new_cov))
    }

    fn mutate(&self, test_case: &TestCase) -> Result<Vec<u8>> {
        // Read the input file
        debug!("Mutating: {}", test_case.filename);
        let input = fs::read(self.get_queue_path(&test_case.filename))?;
        let mut rng = rand::thread_rng();
        let mut result = input.to_vec();
        
        if result.len() == 0 {
            return Ok(result)
        }

        // Choose mutation strategy:
        // 1. Bit flip (30% chance)
        // 2. Byte replacement (20% chance)
        // 3. Delete consecutive bytes (25% chance)
        // 4. Clone/insert bytes (25% chance)
        let strategy = rng.gen_range(0..100);
        
        if strategy < 30 {
            // Strategy 1: Flip a random bit
            let pos = rng.gen_range(0..result.len());
            let bit = rng.gen_range(0..8);
            result[pos] ^= 1 << bit;
        } else if strategy < 50 {
            // Strategy 2: Replace with random byte
            let pos = rng.gen_range(0..result.len());
            result[pos] = rng.gen();
        } else if strategy < 75 {
            // Strategy 3: Delete consecutive bytes
            if result.len() > 1 {  // Only delete if we have at least 2 bytes
                let delete_len = rng.gen_range(1..=std::cmp::min(8, result.len())); // Delete 1-8 bytes
                let start_pos = rng.gen_range(0..=result.len() - delete_len);
                result.drain(start_pos..start_pos + delete_len);
            }
        } else {
            // Strategy 4: Clone/insert bytes
            let chunk_len = rng.gen_range(1..=std::cmp::min(16, result.len())); // Clone/insert 1-16 bytes
            let insert_pos = rng.gen_range(0..=result.len());

            if rng.gen_bool(0.75) { // 75% chance to clone existing bytes
                if result.len() >= chunk_len {
                    // Pick a random source position to clone from
                    let src_pos = rng.gen_range(0..=result.len() - chunk_len);
                    let chunk: Vec<u8> = result[src_pos..src_pos + chunk_len].to_vec();
                    result.splice(insert_pos..insert_pos, chunk);
                }
            } else { // 25% chance to insert constant bytes
                let constant_byte = rng.gen(); // Generate a random constant byte
                let chunk = vec![constant_byte; chunk_len];
                result.splice(insert_pos..insert_pos, chunk);
            }
        }
        
        Ok(result)
    }

    fn fuzz_one_level(&mut self) -> Result<()> {
        while let Some(test_case) = self.queue.pop_front() {
            info!("Fuzzing: {}", test_case.filename);
            
            match self.mutate(&test_case) {
                Ok(mutated) => {
                    match self.run_and_get_coverage(&mutated) {
                        Ok((_path, trigger_new_cov)) => {
                            let filename = self.save_to_queue(&mutated, trigger_new_cov)?;
                            if trigger_new_cov {
                                self.queue.push_back(TestCase { 
                                    filename,
                                });
                            }
                        }
                        Err(e) => {
                            error!("Error running mutated test case from '{}': {}", test_case.filename, e);
                            continue;
                        }
                    }
                }
                Err(e) => {
                    error!("Error during mutation of '{}': {}", test_case.filename, e);
                    continue;
                }
            }
        }
        Ok(())
    }

    fn fuzz_loop(&mut self) -> Result<()> {
        loop {
            self.fuzz_one_level()?;
            self.load_queue()?;
            self.stats.level += 1;
            // break Ok(());
        }
    }

    fn load_queue(&mut self) -> Result<()> {
        for entry in fs::read_dir(&self.queue_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                // Get just the filename component, not the full path
                if let Some(filename) = entry.path().file_name() {
                    if let Some(filename_str) = filename.to_str() {
                        self.queue.push_back(TestCase {
                            filename: filename_str.to_string(),
                        });
                    }
                }
            }
        }
        Ok(())
    }

    fn load_initial_seeds(&mut self) -> Result<()> {
        for entry in fs::read_dir(&self.args.input_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                info!("Loading seed file: {}", entry.path().display());
                let data = fs::read(entry.path()).unwrap();

                match self.run_and_get_coverage(&data) {
                    Ok((path, triggers_new_cov)) => {
                        let filename = self.save_to_queue(&data, triggers_new_cov)?;
                        if triggers_new_cov {
                            self.coverage.update_from_path(&path);
                            self.queue.push_back(TestCase { 
                                filename,
                            });
                            info!("Loaded seed file: {}", entry.path().display());
                            debug!("Path: {:?}", path);
                        } else {
                            warn!("Warning: Initial test case '{}' doesn't trigger new coverage. Perhaps useless?", entry.path().display());
                        }
                    }
                    Err(e) => {
                        error!("Error running seed file '{}': {}", entry.path().display(), e);
                    }
                }
            }
        }
        Ok(())
    }

    fn print_status(&self) {
        let elapsed = self.stats.start_time.map(|t| t.elapsed()).unwrap_or_default();
        let hours = elapsed.as_secs() / 3600;
        let minutes = (elapsed.as_secs() % 3600) / 60;
        let seconds = elapsed.as_secs() % 60;

        println!("\x1B[2J\x1B[1;1H"); // Clear screen and move cursor to top
        println!("=== Fuzzer Status ===");
        println!("Runtime: {:02}:{:02}:{:02}", hours, minutes, seconds);
        println!("Total executions: {}", self.stats.total_executions);
        println!("New coverage found: {}", self.stats.new_coverage_count);
        println!("Crashes found: {}", self.stats.crash_count);
        println!("Exec/s: {:.2}", self.stats.total_executions as f64 / elapsed.as_secs_f64());
        println!("Queue size: {}", self.queue.len());
        println!("Level: {}", self.stats.level);
    }

} 
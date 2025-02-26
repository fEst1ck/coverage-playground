mod error;

use std::{
    collections::VecDeque,
    fs::{self, File},
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
    os::unix::process::ExitStatusExt,
};

use memmap2::MmapOptions;
use rand::Rng;

use crate::{
    cli::Args,
    coverage::{CoverageMetric, create_coverage_metric},
};
pub use error::{FuzzerError, Result};

const COVERAGE_SHM_PATH: &str = "/tmp/coverage_shm.bin";

/// Test case representation
#[derive(Clone)]
struct TestCase {
    /// Name of the file in the queue directory
    filename: String,
}

pub struct Fuzzer {
    args: Args,
    queue: VecDeque<TestCase>,
    coverage: Box<dyn CoverageMetric>,
    uses_file_input: bool,
    queue_dir: PathBuf,      // Directory for storing queue files
    crashes_dir: PathBuf,    // Directory for storing crashes
    next_id: usize,          // Counter for generating unique IDs
}

impl Fuzzer {
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

        Ok(Self {
            coverage: create_coverage_metric(args.coverage_type),
            args,
            queue: VecDeque::new(),
            uses_file_input,
            queue_dir,
            crashes_dir,
            next_id: 0,
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

    fn save_to_queue(&mut self, data: &[u8]) -> Result<String> {
        let filename = format!("id:{:06}", self.next_id);
        self.next_id += 1;

        let path = self.get_queue_path(&filename);
        fs::write(path, data)?;

        Ok(filename)
    }

    fn save_crash(&mut self, data: &[u8], signal: i32) -> Result<()> {
        let filename = format!("crash:{:06},sig:{}", self.next_id, signal);
        let path = self.get_crash_path(&filename);
        fs::write(path, data)?;
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
        // Prepare command
        let mut cmd = Command::new(&self.args.target_cmd[0]);
        
        // Add arguments, replacing @@ with temp file path if needed
        for arg in &self.args.target_cmd[1..] {
            if arg == "@@" {
                // Create temporary file for input
                let mut temp = tempfile::NamedTempFile::new()?;
                temp.write_all(input)?;
                cmd.arg(temp.path());
            } else {
                cmd.arg(arg);
            }
        }

        // Configure stdio
        if !self.uses_file_input {
            cmd.stdin(Stdio::piped());
        }
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());

        // Run the target
        let mut child = cmd.spawn()?;
        
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
                        eprintln!("Warning: Target terminated by unhandled signal: {}", signal);
                        // Continue fuzzing
                    }
                }
            }
        }

        // Read coverage data
        let coverage_file = File::open(COVERAGE_SHM_PATH)?;
        let mmap = unsafe { MmapOptions::new().map(&coverage_file)? };
        
        let mut path = Vec::new();
        if mmap.len() >= 4 {
            let len = u32::from_ne_bytes(mmap[0..4].try_into().unwrap()) as usize;
            if mmap.len() >= 4 + len * 4 {
                for i in 0..len {
                    let offset = 4 + i * 4;
                    let block_id = u32::from_ne_bytes(
                        mmap[offset..offset + 4].try_into().unwrap()
                    );
                    path.push(block_id);
                }
            }
        }

        // Check if this path triggers new coverage
        let trigger_new_cov = self.coverage.has_new_coverage(&path);

        Ok((path, trigger_new_cov))
    }

    fn mutate(&self, test_case: &TestCase) -> Result<Vec<u8>> {
        // Read the input file
        let input = fs::read(self.get_queue_path(&test_case.filename))?;
        let mut rng = rand::rng();
        let mut result = input.to_vec();
        
        // Simple mutation strategy:
        // 1. Pick a random byte
        // 2. Either flip a random bit or replace with random byte
        let pos = rng.random_range(0..result.len());
        if rng.random_bool(0.5) {
            // Flip a random bit
            let bit = rng.random_range(0..8);
            result[pos] ^= 1 << bit;
        } else {
            // Replace with random byte
            result[pos] = rng.random();
        }
        
        Ok(result)
    }

    fn fuzz_loop(&mut self) -> Result<()> {
        while let Some(test_case) = self.queue.pop_front() {
            // Keep original in queue for future mutations
            self.queue.push_back(test_case.clone());

            // Perform one mutation
            match self.mutate(&test_case) {
                Ok(mutated) => {
                    if let Ok((path, trigger_new_cov)) = self.run_and_get_coverage(&mutated) {
                        if trigger_new_cov {
                            // Update coverage
                            self.coverage.update_from_path(&path);
                            // Save to queue and add to queue
                            let filename = self.save_to_queue(&mutated)?;
                            self.queue.push_back(TestCase { 
                                filename,
                            });
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error during mutation: {}", e);
                    continue;
                }
            }
        }
        Ok(())
    }

    fn load_initial_seeds(&mut self) -> Result<()> {
        for entry in fs::read_dir(&self.args.input_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let data = fs::read(entry.path())?;
                if let Ok((path, triggers_new_cov)) = self.run_and_get_coverage(&data) {
                    if triggers_new_cov {
                        // Update coverage
                        self.coverage.update_from_path(&path);
                        // Save to queue and add to queue
                        let filename = self.save_to_queue(&data)?;
                        self.queue.push_back(TestCase { 
                            filename,
                        });
                    } else {
                        eprintln!("Warning: Initial test case '{}' doesn't trigger new coverage. Perhaps useless?", entry.path().display());
                    }
                }
            }
        }
        Ok(())
    }
} 
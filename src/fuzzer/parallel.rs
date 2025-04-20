use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use anyhow::{Ok, Result};
use log::{error, info, warn};

use crate::{cli::Args, fuzzer::Fuzzer};

/// The interval to sync the fuzzer instances in seconds
const SYNC_INTERVAL: u64 = 60;

/// Parallel fuzzer that manages multiple fuzzer instances
pub struct ParallelFuzzer {
    /// Number of parallel instances
    num_instances: usize,
    /// Fuzzer instances
    instances: Vec<Arc<Mutex<Fuzzer>>>,
}

impl ParallelFuzzer {
    /// Create a new parallel fuzzer
    pub fn new(args: Args, num_instances: usize) -> Result<Self> {
        // Create base output directory if it doesn't exist
        std::fs::create_dir_all(&args.output_dir)?;

        // Create instance-specific output directories
        let mut instances = Vec::with_capacity(num_instances);
        for i in 0..num_instances {
            let mut instance_args = args.clone();
            instance_args.output_dir = args.output_dir.join(format!("instance_{}", i));
            instances.push(Arc::new(Mutex::new(Fuzzer::new(instance_args, i)?)));
        }

        Ok(Self {
            num_instances,
            instances,
        })
    }

    /// Run the parallel fuzzer
    pub fn run(&mut self) -> Result<()> {
        info!(
            "Starting parallel fuzzing with {} instances",
            self.num_instances
        );

        // Spawn worker threads
        let mut handles = Vec::with_capacity(self.num_instances);
        for i in 0..self.num_instances {
            let instance = self.instances[i].clone();
            handles.push(thread::spawn(move || {
                {
                    let mut fuzzer = instance.lock().unwrap();
                    fuzzer.load_initial_seeds()?;
                }
                loop {
                    fuzz_one_level(&instance)?;
                    let mut fuzzer = instance.lock().unwrap();
                    fuzzer.load_queue()?;
                    fuzzer.stats.level += 1;
                }
                Ok(())
            }));
        }

        // Spawn sync thread
        let sync_interval = Duration::from_secs(SYNC_INTERVAL);
        let instances = self.instances.clone();
        let sync_handle = thread::spawn(move || loop {
            thread::sleep(sync_interval);
            if let Err(e) = sync_seed_pools(&instances) {
                warn!("Failed to sync queues: {}", e);
            }
        });

        // Wait for all workers to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Stop sync thread
        sync_handle.join().unwrap();

        Ok(())
    }
}

fn fuzz_one_level(fuzzer: &Arc<Mutex<Fuzzer>>) -> Result<()> {
    loop {
        let mut fuzzer = fuzzer.lock().unwrap();
        if let Some(test_case) = fuzzer.next_test_case() {
            if let Err(e) = fuzzer.fuzz_one(&test_case) {
                error!("Error fuzzing test case: {}", e);
            }
        } else {
            break;
        }
        drop(fuzzer);
    }
    Ok(())
}

fn sync_seed_pools(instances: &[Arc<Mutex<Fuzzer>>]) -> Result<()> {
    for i in 0..instances.len() {
        let mut fuzzer = instances[i].lock().unwrap();
        for j in 0..instances.len() {
            if i != j {
                let other = instances[j].lock().unwrap();
                fuzzer.sync_seed_pool(&other)?;
            }
        }
    }
    Ok(())
}

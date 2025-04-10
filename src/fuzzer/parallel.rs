use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use anyhow::Result;
use log::{info, warn};

use crate::{
    cli::Args,
    fuzzer::Fuzzer,
};

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
        info!("Starting parallel fuzzing with {} instances", self.num_instances);

        // Spawn worker threads
        let mut handles = Vec::with_capacity(self.num_instances);
        for i in 0..self.num_instances {
            let instance = self.instances[i].clone();
            handles.push(thread::spawn(move || {
                let mut fuzzer = instance.lock().unwrap();
                match fuzzer.run() {
                    Ok(_) => info!("Instance {} completed successfully", i),
                    Err(e) => warn!("Instance {} failed: {}", i, e),
                }
            }));
        }

        // Spawn sync thread
        let sync_interval = Duration::from_secs(SYNC_INTERVAL);
        let instances = self.instances.clone();
        let sync_handle = thread::spawn(move || {
            loop {
                thread::sleep(sync_interval);
                if let Err(e) = Self::sync_seed_pools(&instances) {
                    warn!("Failed to sync queues: {}", e);
                }
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

    /// Sync seed pools between instances
    fn sync_seed_pools(instances: &[Arc<Mutex<Fuzzer>>]) -> Result<()> {
        // For each pair of instances, sync their seed pools
        for i in 0..instances.len() {
            for j in 0..instances.len() {
                if i != j {
                    let fuzzer1 = instances[i].lock().unwrap();
                    let mut fuzzer2 = instances[j].lock().unwrap();
                    if let Err(e) = fuzzer2.sync_seed_pool(&fuzzer1) {
                        warn!("Failed to sync seed pool from instance {} to {}: {}", i, j, e);
                    }
                }
            }
        }
        Ok(())
    }
} 
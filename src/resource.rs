use std::time::Duration;

use sysinfo::{Pid, System};

pub fn start_resource_logger() {
    tokio::spawn(async move {
        let pid = std::process::id();
        let mut sys = System::new_all();

        loop {
            sys.refresh_process(Pid::from_u32(pid));

            if let Some(proc) = sys.process(Pid::from_u32(pid)) {
                let mem_mb = proc.memory() as f64 / 1024.0 / 1024.0; // KB → MB
                let cpu = proc.cpu_usage();
                println!("[STATS] CPU: {:.2}% | Memory: {:.2} MB", cpu, mem_mb);
            }

            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
}

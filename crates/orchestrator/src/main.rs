mod prd;
mod ralph;

use ralph::RalphOrchestrator;
use std::env;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    
    let mut prd_path = "prd.json".to_string();
    let mut max_iterations: u32 = 10;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--prd" => {
                if i + 1 < args.len() {
                    prd_path = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("Error: --prd requires a path argument");
                    std::process::exit(1);
                }
            }
            "--max-iterations" => {
                if i + 1 < args.len() {
                    max_iterations = args[i + 1].parse().unwrap_or(10);
                    i += 2;
                } else {
                    eprintln!("Error: --max-iterations requires a number");
                    std::process::exit(1);
                }
            }
            "--help" | "-h" => {
                println!("Ralph Orchestrator");
                println!();
                println!("Usage: ralph [OPTIONS]");
                println!();
                println!("Options:");
                println!("  --prd <PATH>           Path to prd.json (default: prd.json)");
                println!("  --max-iterations <N>   Max iterations (default: 10)");
                println!("  --help, -h             Show this help");
                return;
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                std::process::exit(1);
            }
        }
    }

    let orchestrator = RalphOrchestrator::new(&prd_path, max_iterations);
    
    match orchestrator.run().await {
        Ok(()) => {
            eprintln!("Orchestrator completed successfully");
        }
        Err(e) => {
            eprintln!("Orchestrator failed: {}", e);
            std::process::exit(1);
        }
    }
}

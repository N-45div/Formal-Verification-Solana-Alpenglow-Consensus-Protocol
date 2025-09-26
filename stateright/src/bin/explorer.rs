//! Alpenglow Stateright Explorer
//!
//! Interactive exploration tool for the Alpenglow consensus protocol model.
//! This tool allows you to explore the state space, run model checking,
//! and analyze protocol behavior.

use alpenglow_verification::*;
use stateright::*;
use std::env;

fn main() {
    env_logger::init();
    
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        println!("Usage: {} <command> [options]", args[0]);
        println!("Commands:");
        println!("  votor <nodes> <byzantine_ratio> <window_size> <max_slot>");
        println!("  rotor <nodes> <byzantine_ratio> <window_size> <max_slot> <gamma> <expansion_ratio>");
        println!("  serve <port>");
        return;
    }

    match args[1].as_str() {
        "votor" => {
            if args.len() != 6 {
                println!("Usage: {} votor <nodes> <byzantine_ratio> <window_size> <max_slot>", args[0]);
                return;
            }
            
            let nodes: usize = args[2].parse().expect("Invalid node count");
            let byzantine_ratio: f64 = args[3].parse().expect("Invalid Byzantine ratio");
            let window_size: u32 = args[4].parse().expect("Invalid window size");
            let max_slot: u32 = args[5].parse().expect("Invalid max slot");
            
            println!("Running Votor model checking...");
            println!("Nodes: {}, Byzantine ratio: {:.1}%, Window size: {}, Max slot: {}", 
                     nodes, byzantine_ratio * 100.0, window_size, max_slot);
            
            let system = VotorSystem::new(nodes, byzantine_ratio, window_size, max_slot);
            let properties_count = system.properties().len();
            
            let checker = system.checker()
                .threads(num_cpus::get())
                .spawn_dfs();
            
            println!("Model checking in progress...");
            let result = checker.join();
            
            println!("Model checking completed!");
            println!("Properties verified: {}", properties_count);
            
            result.assert_properties();
            println!("✅ All properties verified successfully!");
        }
        
        "rotor" => {
            if args.len() != 8 {
                println!("Usage: {} rotor <nodes> <byzantine_ratio> <window_size> <max_slot> <gamma> <expansion_ratio>", args[0]);
                return;
            }
            
            let nodes: usize = args[2].parse().expect("Invalid node count");
            let byzantine_ratio: f64 = args[3].parse().expect("Invalid Byzantine ratio");
            let window_size: u32 = args[4].parse().expect("Invalid window size");
            let max_slot: u32 = args[5].parse().expect("Invalid max slot");
            let gamma: u32 = args[6].parse().expect("Invalid gamma");
            let expansion_ratio: f64 = args[7].parse().expect("Invalid expansion ratio");
            
            println!("Running Rotor model checking...");
            println!("Nodes: {}, Byzantine ratio: {:.1}%, Window size: {}, Max slot: {}", 
                     nodes, byzantine_ratio * 100.0, window_size, max_slot);
            println!("Erasure coding: γ={}, κ={:.2}", gamma, expansion_ratio);
            
            let system = RotorSystem::new(nodes, byzantine_ratio, window_size, max_slot, gamma, expansion_ratio);
            let properties_count = system.properties().len();
            
            let checker = system.checker()
                .threads(num_cpus::get())
                .spawn_dfs();
            
            println!("Model checking in progress...");
            let result = checker.join();
            
            println!("Model checking completed!");
            println!("Properties verified: {}", properties_count);
            
            result.assert_properties();
            println!("✅ All properties verified successfully!");
        }
        
        "serve" => {
            let port: u16 = args.get(2)
                .and_then(|s| s.parse().ok())
                .unwrap_or(3000);
            
            println!("Starting Stateright Explorer web interface on port {}", port);
            println!("Open http://localhost:{} in your browser", port);
            
            // Create a default Votor system for exploration
            let system = VotorSystem::new(4, 0.2, 2, 5);
            
            system.checker()
                .threads(1)
                .serve(format!("0.0.0.0:{}", port));
        }
        
        _ => {
            println!("Unknown command: {}", args[1]);
            println!("Available commands: votor, rotor, serve");
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_votor_integration() {
        let system = VotorSystem::new(3, 0.2, 2, 2);
        let result = system.checker()
            .threads(1)
            .spawn_dfs()
            .join();
        
        // Model checking completed successfully
        result.assert_properties();
    }

    #[test]
    fn test_rotor_integration() {
        let system = RotorSystem::new(3, 0.2, 2, 1, 2, 2.0);
        let result = system.checker()
            .threads(1)
            .spawn_dfs()
            .join();
        
        // Model checking completed successfully
        result.assert_properties();
    }

    #[test]
    fn test_byzantine_tolerance_limits() {
        // Test that we respect the 20% Byzantine limit
        let system = VotorSystem::new(10, 0.15, 3, 3);
        let byzantine_stake: u32 = system.base.byzantine_nodes.iter()
            .map(|&n| system.base.stake(n))
            .sum();
        let total_stake = system.base.total_stake();
        
        // Verify Byzantine stake is less than 20%
        assert!(byzantine_stake * 5 < total_stake);
    }

    #[test]
    fn test_erasure_coding_resilience() {
        // Test Lemma 7: κ > 5/3 condition
        let system = RotorSystem::new(5, 0.2, 2, 2, 3, 2.0);
        assert!(system.expansion_ratio > 5.0 / 3.0);
        
        // Test that we can reconstruct with γ shreds
        assert!(system.big_gamma >= system.gamma);
        assert_eq!(system.big_gamma, (system.gamma as f64 * system.expansion_ratio) as u32);
    }
}

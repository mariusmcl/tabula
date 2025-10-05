use tabula_blockchain::Blockchain;

fn main() {
    println!("🚀 Welcome to Tabula Blockchain - CLI Mode\n");
    println!("For GUI mode, run: cargo tauri dev\n");

    println!("Creating blockchain with difficulty level 2...");
    let mut blockchain = Blockchain::new(2);

    println!("\nAdding block 1...");
    blockchain.add_block("Hello, World!".to_string());

    println!("\nAdding block 2...");
    blockchain.add_block("This is a simple blockchain".to_string());

    println!("\nAdding block 3...");
    blockchain.add_block("Built with Rust 🦀".to_string());

    println!("\n=== BLOCKCHAIN ===\n");
    for block in blockchain.get_chain() {
        println!("Block #{}:", block.index);
        println!("  Timestamp: {}", block.timestamp);
        println!("  Data: {}", block.data);
        println!("  Previous Hash: {}", block.previous_hash);
        println!("  Hash: {}", block.hash);
        println!("  Nonce: {}", block.nonce);
        println!();
    }

    println!("=== VALIDATION ===");
    if blockchain.is_chain_valid() {
        println!("✅ Blockchain is valid!");
    } else {
        println!("❌ Blockchain is invalid!");
    }
}

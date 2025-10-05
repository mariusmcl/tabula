use std::fs::File;
use std::io::{Read, Write, BufReader, BufWriter};

fn main() {
    println!("🚀 Binary Data Read/Write Example\n");
    
    // Example 1: Basic binary data write and read
    basic_binary_example();
    
    // Example 2: What happens when reading more than written
    overflow_read_example();
    
    // Example 3: Structured data serialization
    structured_data_example();
}

fn basic_binary_example() {
    println!("=== Basic Binary Data Example ===");
    
    let file_path = "test_data.bin";
    
    // Data to write
    let data_to_write = vec![0x48, 0x65, 0x6C, 0x6C, 0x6F]; // "Hello" in bytes
    
    // Write binary data
    match write_binary_data(file_path, &data_to_write) {
        Ok(()) => println!("✅ Successfully wrote {} bytes to {}", data_to_write.len(), file_path),
        Err(e) => println!("❌ Error writing data: {}", e),
    }
    
    // Read the same amount of data back
    match read_binary_data(file_path, data_to_write.len()) {
        Ok(read_data) => {
            println!("✅ Successfully read {} bytes: {:?}", read_data.len(), read_data);
            println!("   As string: {}", String::from_utf8_lossy(&read_data));
        },
        Err(e) => println!("❌ Error reading data: {}", e),
    }
    
    println!();
}

fn overflow_read_example() {
    println!("=== Overflow Read Example ===");
    
    let file_path = "small_data.bin";
    let small_data = vec![0x01, 0x02, 0x03]; // Only 3 bytes
    
    // Write small data
    if let Err(e) = write_binary_data(file_path, &small_data) {
        println!("❌ Error writing: {}", e);
        return;
    }
    
    println!("Wrote {} bytes: {:?}", small_data.len(), small_data);
    
    // Try to read more than we wrote
    println!("Trying to read 10 bytes from a 3-byte file...");
    match read_binary_data(file_path, 10) {
        Ok(read_data) => {
            println!("✅ Read {} bytes: {:?}", read_data.len(), read_data);
            println!("   Note: Only got {} bytes (file size), rest are zeros", small_data.len());
        },
        Err(e) => println!("❌ Error reading: {}", e),
    }
    
    // Try to read exactly what we wrote
    println!("\nTrying to read exactly 3 bytes...");
    match read_binary_data(file_path, 3) {
        Ok(read_data) => {
            println!("✅ Read {} bytes: {:?}", read_data.len(), read_data);
        },
        Err(e) => println!("❌ Error reading: {}", e),
    }
    
    println!();
}

fn structured_data_example() {
    println!("=== Structured Data Example ===");
    
    let file_path = "structured_data.bin";
    
    // Create some structured data
    let numbers = vec![42u32, 1337u32, 0xDEADBEEFu32];
    let text = "Hello, Binary World!";
    
    // Write structured data
    match write_structured_data(file_path, &numbers, text) {
        Ok(()) => println!("✅ Successfully wrote structured data"),
        Err(e) => println!("❌ Error writing structured data: {}", e),
    }
    
    // Read it back
    match read_structured_data(file_path) {
        Ok((read_numbers, read_text)) => {
            println!("✅ Successfully read structured data:");
            println!("   Numbers: {:?}", read_numbers);
            println!("   Text: {}", read_text);
        },
        Err(e) => println!("❌ Error reading structured data: {}", e),
    }
    
    println!();
}

fn write_binary_data(path: &str, data: &[u8]) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(data)?;
    Ok(())
}

fn read_binary_data(path: &str, size: usize) -> std::io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut buffer = vec![0u8; size];
    
    // Read exactly 'size' bytes, or as many as available
    let bytes_read = file.read(&mut buffer)?;
    
    // If we read fewer bytes than requested, truncate the buffer
    buffer.truncate(bytes_read);
    
    Ok(buffer)
}

fn write_structured_data(path: &str, numbers: &[u32], text: &str) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    
    // Write count of numbers
    file.write_all(&(numbers.len() as u32).to_le_bytes())?;
    
    // Write each number as little-endian bytes
    for &num in numbers {
        file.write_all(&num.to_le_bytes())?;
    }
    
    // Write text length and text
    let text_bytes = text.as_bytes();
    file.write_all(&(text_bytes.len() as u32).to_le_bytes())?;
    file.write_all(text_bytes)?;
    
    Ok(())
}

fn read_structured_data(path: &str) -> std::io::Result<(Vec<u32>, String)> {
    let mut file = BufReader::new(File::open(path)?);
    
    // Read count of numbers
    let mut count_buf = [0u8; 4];
    file.read_exact(&mut count_buf)?;
    let count = u32::from_le_bytes(count_buf) as usize;
    
    // Read numbers
    let mut numbers = Vec::new();
    for _ in 0..count {
        let mut num_buf = [0u8; 4];
        file.read_exact(&mut num_buf)?;
        numbers.push(u32::from_le_bytes(num_buf));
    }
    
    // Read text length
    let mut text_len_buf = [0u8; 4];
    file.read_exact(&mut text_len_buf)?;
    let text_len = u32::from_le_bytes(text_len_buf) as usize;
    
    // Read text
    let mut text_bytes = vec![0u8; text_len];
    file.read_exact(&mut text_bytes)?;
    let text = String::from_utf8(text_bytes)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    
    Ok((numbers, text))
}

/*
fn main3() {
    println!("🚀 Welcome to Tabula Blockchain 123 - CLI Mode\n");
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
 */
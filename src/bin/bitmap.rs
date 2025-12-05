use anyhow::{Context, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashSet;
use std::fs;
use std::str::FromStr;

fn main() -> Result<()> {
    println!("MEV Guard Bitmap Generator");
    println!("==========================\n");

    // Read blocklist from file
    let blocklist_path = "blocklist.txt";
    println!("Reading blocklist from {}", blocklist_path);
    let blocklist_content = fs::read_to_string(blocklist_path)
        .context("Failed to read blocklist.txt")?;

    let blocklist: HashSet<Pubkey> = blocklist_content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            Pubkey::from_str(line.trim())
                .with_context(|| format!("Invalid pubkey in blocklist: {}", line))
        })
        .collect::<Result<HashSet<_>>>()?;

    println!("Loaded {} addresses in blocklist\n", blocklist.len());

    // Connect to Solana mainnet
    let rpc_url = "https://api.mainnet-beta.solana.com";
    println!("Connecting to Solana mainnet: {}", rpc_url);
    let client = RpcClient::new(rpc_url.to_string());

    // Get current epoch
    let epoch_info = client.get_epoch_info()
        .context("Failed to get epoch info")?;
    let current_epoch = epoch_info.epoch;
    println!("Current epoch: {}\n", current_epoch);

    // Get leader schedule for current epoch
    println!("Fetching leader schedule for epoch {}...", current_epoch);
    let leader_schedule = client.get_leader_schedule(None)
        .context("Failed to get leader schedule")?
        .context("Leader schedule not available")?;

    // Collect all slots with their leaders
    let mut slot_leaders: Vec<(u64, Pubkey)> = Vec::new();
    for (pubkey_str, slots) in leader_schedule.iter() {
        let pubkey = Pubkey::from_str(pubkey_str)
            .with_context(|| format!("Invalid pubkey in leader schedule: {}", pubkey_str))?;
        for &slot in slots {
            slot_leaders.push((slot as u64, pubkey));
        }
    }

    // Sort by slot number
    slot_leaders.sort_by_key(|(slot, _)| *slot);

    println!("Total slots in schedule: {}", slot_leaders.len());

    // Sample every 4th slot (one per leader rotation)
    let sampled_slots: Vec<(u64, Pubkey)> = slot_leaders
        .iter()
        .step_by(4)
        .copied()
        .collect();

    println!("Sampled slots (every 4th): {}\n", sampled_slots.len());

    // Create bitmap: 1 if leader is blocklisted, 0 otherwise
    let mut bitmap_bits: Vec<bool> = Vec::new();
    let mut blocklisted_count = 0;

    for (slot, leader) in &sampled_slots {
        let is_blocklisted = blocklist.contains(leader);
        bitmap_bits.push(is_blocklisted);

        if is_blocklisted {
            blocklisted_count += 1;
            println!("Slot {}: {} [BLOCKLISTED]", slot, leader);
        }
    }

    println!("\nBitmap statistics:");
    println!("  Total sampled slots: {}", bitmap_bits.len());
    println!("  Blocklisted leaders: {}", blocklisted_count);
    println!("  Good leaders: {}", bitmap_bits.len() - blocklisted_count);
    println!("  Blocklist rate: {:.2}%", (blocklisted_count as f64 / bitmap_bits.len() as f64) * 100.0);

    // Convert bits to bytes (pack 8 bits per byte)
    let mut bitmap_bytes: Vec<u8> = Vec::new();
    for chunk in bitmap_bits.chunks(8) {
        let mut byte = 0u8;
        for (i, &bit) in chunk.iter().enumerate() {
            if bit {
                byte |= 1 << (7 - i); // MSB first
            }
        }
        bitmap_bytes.push(byte);
    }

    // Save raw bytes to file
    let output_filename = format!("epoch-{}-bitmap.bin", current_epoch);
    fs::write(&output_filename, &bitmap_bytes)
        .context("Failed to write bitmap file")?;

    println!("\nBitmap saved to: {}", output_filename);
    println!("Bitmap size: {} bytes ({} bits)", bitmap_bytes.len(), bitmap_bits.len());
    println!("Hex preview: {}...", hex::encode(&bitmap_bytes[..bitmap_bytes.len().min(32)]));

    Ok(())
}

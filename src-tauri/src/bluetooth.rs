use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Manager, Adapter};
use std::time::Duration;
use tokio::time;
use once_cell::sync::OnceCell;

static ADAPTER: OnceCell<Adapter> = OnceCell::new();

async fn get_central() -> Result<Adapter, String> {
    if let Some(adapter) = ADAPTER.get() {
        return Ok(adapter.clone());
    }

    let manager = Manager::new().await.map_err(|e| e.to_string())?;
    let adapters = manager.adapters().await.map_err(|e| e.to_string())?;
    let adapter = adapters.into_iter().next().ok_or("No Bluetooth adapters found")?;
    
    // We don't mind if multiple threads try to set it, OnceCell::set will fail if already set
    let _ = ADAPTER.set(adapter.clone());
    Ok(adapter)
}

/// Spins up the system Bluetooth manager, scans for 5 seconds, and returns discovered devices.
#[tauri::command]
pub async fn scan_bluetooth_printers() -> Result<Vec<String>, String> {
    let central = get_central().await?;

    // Start scanning
    central.start_scan(ScanFilter::default()).await.map_err(|e| e.to_string())?;

    // Wait 5 seconds
    time::sleep(Duration::from_secs(5)).await;

    // Collect results
    let mut discovered = Vec::new();
    let peripherals = central.peripherals().await.map_err(|e| e.to_string())?;
    
    for peripheral in peripherals {
        if let Ok(Some(properties)) = peripheral.properties().await {
            let name = properties.local_name.unwrap_or_else(|| "Unknown Device".to_string());
            let address = peripheral.id().to_string();
            // We just format it nicely for the frontend: "Device_Name (00:11:22...)"
            discovered.push(format!("{} ({})", name, address));
        }
    }

    Ok(discovered)
}

/// Connects to a specific Bluetooth MAC address/UUID to verify and establish an OS pairing, then disconnects.
#[tauri::command]
pub async fn pair_bluetooth_printer(address: String) -> Result<(), String> {
    println!("[Bluetooth] Starting OS pairing sequence for address: {}", address);
    
    let mac = extract_mac(&address).ok_or("Invalid Bluetooth address format")?;
    
    let central = get_central().await?;

    // Brief scan to find the device if not already in cache
    central.start_scan(ScanFilter::default()).await.map_err(|e| e.to_string())?;
    time::sleep(Duration::from_secs(2)).await;
    let _ = central.stop_scan().await;

    let peripherals = central.peripherals().await.map_err(|e| e.to_string())?;
    
    let target = peripherals.into_iter().find(|p| p.id().to_string() == mac).ok_or_else(|| {
        "Printer not found or out of range for pairing"
    })?;

    // Connect to explicitly trigger macOS/Windows pairing prompt, or just verify reachability
    if !target.is_connected().await.unwrap_or(false) {
        println!("[Bluetooth] Connecting to target for pairing...");
        target.connect().await.map_err(|e| format!("Failed to pair/connect: {}", e))?;
        println!("[Bluetooth] Successfully paired/connected to target");
    }

    // Try discovering services just to ensure the connection is fully negotiated
    target.discover_services().await.map_err(|e| format!("Failed to discover services during pairing: {}", e))?;

    println!("[Bluetooth] Leaving OS connection open for active pairing status...");
    
    Ok(())
}

/// Connects to a specific Bluetooth MAC address and writes the payload to the first writeable characteristic
#[tauri::command]
pub async fn print_bluetooth_payload(address: String, payload: Vec<u8>) -> Result<(), String> {
    println!("[Bluetooth] Starting print payload to address: {}", address);
    
    // Parse the MAC address from the selected string format "Name (MAC)"
    let mac = extract_mac(&address).ok_or("Invalid Bluetooth address format")?;
    println!("[Bluetooth] Extracted MAC: {}", mac);

    let central = get_central().await?;

    let mut peripherals = central.peripherals().await.unwrap_or_default();
    let mut target_opt = peripherals.into_iter().find(|p| p.id().to_string() == mac);

    if target_opt.is_none() {
        println!("[Bluetooth] Target not found in immediate cache, starting brief 2-second scan...");
        // We MUST scan for at least a brief moment so the OS caches the peripheral if we haven't scanned recently
        central.start_scan(ScanFilter::default()).await.map_err(|e| {
            println!("[Bluetooth] Failed to start scan: {}", e);
            e.to_string()
        })?;
        time::sleep(Duration::from_secs(2)).await;
        let _ = central.stop_scan().await;
        println!("[Bluetooth] Scan finished, scanning for target peripheral in cache");

        // Discover the target peripheral again
        peripherals = central.peripherals().await.map_err(|e| {
            println!("[Bluetooth] Failed to get peripherals from central: {}", e);
            e.to_string()
        })?;
        
        target_opt = peripherals.into_iter().find(|p| p.id().to_string() == mac);
    } else {
        println!("[Bluetooth] Target found in immediate cache, skipping scan");
    }

    let target = target_opt.ok_or_else(|| {
        println!("[Bluetooth] Printer not found or out of range after scan");
        "Printer not found or out of range"
    })?;

    // Connect
    if !target.is_connected().await.unwrap_or(false) {
        println!("[Bluetooth] Target is not connected, attempting to connect...");
        target.connect().await.map_err(|e| {
            println!("[Bluetooth] Failed to connect to target: {}", e);
            format!("Failed to connect: {}", e)
        })?;
        println!("[Bluetooth] Successfully connected to target");
    } else {
        println!("[Bluetooth] Target was already connected");
    }

    println!("[Bluetooth] Discovering services on target...");
    // Discover services & characteristics
    target.discover_services().await.map_err(|e| {
        println!("[Bluetooth] Failed to discover services: {}", e);
        format!("Failed to discover services: {}", e)
    })?;
    
    println!("[Bluetooth] Searching for writable characteristic...");
    // Find a characteristic that supports Writing without response (or with response)
    let chars = target.characteristics();
    
    // Prefer WRITE_WITHOUT_RESPONSE
    let mut write_char_opt = chars.iter().find(|c| {
        c.properties.contains(btleplug::api::CharPropFlags::WRITE_WITHOUT_RESPONSE)
    });
    
    // Fallback to WRITE
    if write_char_opt.is_none() {
        write_char_opt = chars.iter().find(|c| {
            c.properties.contains(btleplug::api::CharPropFlags::WRITE)
        });
    }

    let write_char = write_char_opt.ok_or_else(|| {
        println!("[Bluetooth] No writable characteristics found");
        "No writable characteristics found on this printer"
    })?;

    // Determine write type
    let write_type = if write_char.properties.contains(btleplug::api::CharPropFlags::WRITE_WITHOUT_RESPONSE) {
        println!("[Bluetooth] Selected WriteType::WithoutResponse");
        WriteType::WithoutResponse
    } else {
        println!("[Bluetooth] Selected WriteType::WithResponse");
        WriteType::WithResponse
    };

    println!("[Bluetooth] Writing {} bytes to the printer in chunks...", payload.len());
    // Write the raw payload in chunks of 20 bytes (standard BLE MTU is 20-23 without negotiation)
    for chunk in payload.chunks(20) {
        target.write(write_char, chunk, write_type).await.map_err(|e| {
            println!("[Bluetooth] Failed to write payload chunk: {}", e);
            format!("Failed to write to printer: {}", e)
        })?;
        // Brief sleep to avoid flooding the buffer, especially if WithoutResponse
        time::sleep(Duration::from_millis(10)).await;
    }

    println!("[Bluetooth] Leaving OS connection open for next print job...");
    
    println!("[Bluetooth] Finished successfully!");
    Ok(())
}

fn extract_mac(address: &str) -> Option<String> {
    // Expected format: "Name (00:11:22...)"
    // Or it might just be the raw MAC if they passed it in directly
    if address.contains("(") && address.contains(")") {
        let start = address.find("(")?;
        let end = address.find(")")?;
        Some(address[start + 1..end].to_string())
    } else {
        Some(address.to_string())
    }
}

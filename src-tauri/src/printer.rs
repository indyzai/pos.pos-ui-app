use std::net::TcpStream;
use std::io::Write;
use std::process::Command;

#[cfg(target_os = "windows")]
pub fn print_windows_usb(printer_name: &str, data: &[u8]) -> Result<(), String> {
    use std::ptr;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Graphics::Printing::{
        ClosePrinter, EndDocPrinter, EndPagePrinter, OpenPrinterW, StartDocPrinterW,
        StartPagePrinter, WritePrinter, DOC_INFO_1W, PRINTER_HANDLE,
    };
    use std::os::windows::ffi::OsStrExt;

    let mut printer_w: Vec<u16> = std::ffi::OsStr::new(printer_name).encode_wide().chain(Some(0)).collect();
    let mut doc_name: Vec<u16> = std::ffi::OsStr::new("POS Receipt").encode_wide().chain(Some(0)).collect();
    let mut data_type: Vec<u16> = std::ffi::OsStr::new("RAW").encode_wide().chain(Some(0)).collect();

    let mut hprinter: PRINTER_HANDLE = PRINTER_HANDLE { Value: std::ptr::null_mut() };

    unsafe {
        if OpenPrinterW(printer_w.as_mut_ptr(), &mut hprinter, ptr::null_mut()) == 0 {
            return Err(format!("Could not open printer {}", printer_name));
        }

        let doc_info = DOC_INFO_1W {
            pDocName: doc_name.as_mut_ptr(),
            pOutputFile: ptr::null_mut(),
            pDatatype: data_type.as_mut_ptr(),
        };

        if StartDocPrinterW(hprinter, 1, &doc_info as *const _ as *const _) == 0 {
            ClosePrinter(hprinter);
            return Err("Failed to start document".to_string());
        }

        if StartPagePrinter(hprinter) == 0 {
            EndDocPrinter(hprinter);
            ClosePrinter(hprinter);
            return Err("Failed to start page".to_string());
        }

        let mut bytes_written: u32 = 0;
        let success = WritePrinter(
            hprinter,
            data.as_ptr() as *const _,
            data.len() as u32,
            &mut bytes_written,
        );

        EndPagePrinter(hprinter);
        EndDocPrinter(hprinter);
        ClosePrinter(hprinter);

        if success == 0 {
            return Err("Failed to write to printer".to_string());
        }
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn print_unix_usb(printer_name: &str, data: &[u8]) -> Result<(), String> {
    use std::fs::File;
    use std::env::temp_dir;

    if printer_name.trim().is_empty() || printer_name.to_lowercase() == "none" || printer_name == "receipt" {
        return Err("No OS printer selected or installed on this system. Please add a printer in your System Settings.".to_string());
    }

    let temp_file_path = temp_dir().join("pos_receipt.bin");
    {
        let mut file = File::create(&temp_file_path).map_err(|e| e.to_string())?;
        file.write_all(data).map_err(|e| e.to_string())?;
    }

    let output = Command::new("lp")
        .args(["-d", printer_name, "-o", "raw", temp_file_path.to_str().unwrap()])
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(err);
    }
    Ok(())
}

#[tauri::command]
pub async fn get_system_printers() -> Result<Vec<String>, String> {
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("powershell")
            .args(["-Command", "Get-Printer | Select-Object -ExpandProperty Name"])
            .output()
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            return Err("Failed to get printers".to_string());
        }

        let result = String::from_utf8_lossy(&output.stdout);
        let printers: Vec<String> = result
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Ok(printers)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let output = Command::new("lpstat")
            .arg("-p")
            .output()
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("No destinations added") {
                return Ok(vec![]);
            }
            return Err(format!("Failed to get printers: {}", stderr));
        }

        let result = String::from_utf8_lossy(&output.stdout);
        // lpstat -p usually outputs lines like: printer HP_LaserJet_1020 is idle.  enabled since...
        let printers: Vec<String> = result
            .lines()
            .filter(|line| line.starts_with("printer "))
            .flat_map(|line| line.split_whitespace().nth(1))
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Ok(printers)
    }
}

#[tauri::command]
pub async fn print_raw_payload(
    printer_type: String, // "usb" or "network"
    address: String,      // IP:Port for network, or Printer Name for usb
    payload: Vec<u8>,
) -> Result<(), String> {
    if payload.is_empty() {
        return Err("Payload is empty".to_string());
    }

    if printer_type.to_lowercase() == "network" {
        let mut stream = TcpStream::connect(&address).map_err(|e| format!("Failed to connect to printer at {}: {}", address, e))?;
        stream.write_all(&payload).map_err(|e| format!("Failed to send data: {}", e))?;
        stream.flush().map_err(|e| e.to_string())?;
        Ok(())
    } else if printer_type.to_lowercase() == "bluetooth" {
        // Route to the btleplug implementation
        crate::bluetooth::print_bluetooth_payload(address, payload).await
    } else if printer_type.to_lowercase() == "usb" {
        #[cfg(target_os = "windows")]
        {
            print_windows_usb(&address, &payload)
        }
        #[cfg(not(target_os = "windows"))]
        {
            print_unix_usb(&address, &payload)
        }
    } else {
        Err(format!("Unknown printer type: {}", printer_type))
    }
}

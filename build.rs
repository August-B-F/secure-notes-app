fn main() {
    #[cfg(target_os = "windows")]
    {
        // Create .ico from PNG for the Windows executable icon
        let png_path = std::path::Path::new("assets/logo.png");
        let ico_path = std::path::Path::new("assets/logo.ico");

        // Convert PNG to ICO if ICO doesn't exist or PNG is newer
        let need_convert = if ico_path.exists() {
            let png_modified = std::fs::metadata(png_path).and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            let ico_modified = std::fs::metadata(ico_path).and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            png_modified > ico_modified
        } else {
            true
        };

        if need_convert && png_path.exists() {
            // Simple PNG-to-ICO: ICO format wraps PNG data directly for sizes >= 48
            // Header: 6 bytes, then 1 entry (16 bytes), then PNG data
            if let Ok(png_data) = std::fs::read(png_path) {
                let size: u8 = 0; // 0 means 256px in ICO format
                let mut ico = Vec::new();
                // ICO header
                ico.extend_from_slice(&[0, 0]); // reserved
                ico.extend_from_slice(&[1, 0]); // type: icon
                ico.extend_from_slice(&[1, 0]); // count: 1 image
                // ICO directory entry
                ico.push(size); // width (0 = 256)
                ico.push(size); // height (0 = 256)
                ico.push(0);    // color palette
                ico.push(0);    // reserved
                ico.extend_from_slice(&[1, 0]); // color planes
                ico.extend_from_slice(&[32, 0]); // bits per pixel
                let data_size = png_data.len() as u32;
                ico.extend_from_slice(&data_size.to_le_bytes()); // image size
                let data_offset: u32 = 6 + 16; // header + 1 entry
                ico.extend_from_slice(&data_offset.to_le_bytes()); // offset
                // PNG data
                ico.extend_from_slice(&png_data);
                let _ = std::fs::write(ico_path, &ico);
            }
        }

        if ico_path.exists() {
            let mut res = winresource::WindowsResource::new();
            res.set_icon("assets/logo.ico");
            res.set("ProductName", "Notes");
            res.set("FileDescription", "Secure Encrypted Notes");
            res.set("CompanyName", "");
            res.set("LegalCopyright", "");
            if let Err(e) = res.compile() {
                eprintln!("winresource error: {}", e);
            }
        }
    }
}

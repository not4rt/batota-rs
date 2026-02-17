# Batota

A lightweight memory scanner and offline game cheat tool for Linux with a GUI interface, inspired by Cheat Engine.

**Batota** (Portuguese for "cheating" or "trickery") is a GameConqueror alternative built with minimal dependencies using Rust and egui.

## Screenshot

![Batota Interface](batota-0_1.png)

*Batota's interface showing memory scanning and address management*

## Features

- Process memory scanning and editing
- Multiple value types support (i32, i64, f32, f64, u8, u16, u32, u64)
- Scan types: exact value, increased, decreased, unchanged, changed
- Real-time memory monitoring and value freezing
- Address bookmarking with descriptions
- Lightweight GUI with minimal dependencies
- Multi-threaded scanning for performance

## Requirements

- Linux operating system
- Root/sudo privileges (required for reading process memory via `/proc/<pid>/mem`)

## Installation

### From Binary

Download the latest release from the [Releases](https://github.com/not4rt/batota-rs/releases) page.

```bash
chmod +x batota
sudo ./batota
```

### From Source

```bash
git clone https://github.com/not4rt/batota-rs.git
cd batota
cargo build --release
sudo ./target/release/batota
```

## Usage

1. Run Batota with sudo privileges:
   ```bash
   sudo ./batota
   ```

2. Click **File → Open process** to select a target process

3. Configure scan parameters:
   - **Value Type**: Select data type (i32, i64, f32, etc.)
   - **Scan Type**: Choose scan method (Exact Value, Increased, Decreased, etc.)
   - **Value**: Enter the value to search for

4. Click **First Scan** to initiate memory scanning

5. Change the value in the target application, then click **Next Scan** to refine results

6. Double-click results to add them to the address list for monitoring/editing

7. Edit values by double-clicking them in the saved addresses table

8. Enable **Frozen** checkbox to lock values in memory

## Technical Details

- **Language**: Rust
- **GUI Framework**: egui/eframe
- **Memory Access**: `/proc/<pid>/mem` with `nix` crate
- **Parallel Scanning**: rayon for multi-threaded operations

## Why Batota?

- **Minimal Dependencies**: Unlike GameConqueror which requires many GTK dependencies
- **Performance**: Multi-threaded scanning with Rust's zero-cost abstractions
- **Simple**: Focused on core memory scanning functionality
- **Native**: Direct memory access without Python overhead

## Security Note

This tool requires elevated privileges to read and write process memory. Use responsibly and only on applications you own or have permission to modify.

## License

This project is provided as-is for educational purposes.

## Contributing

Contributions are welcome! Feel free to open issues or submit pull requests.

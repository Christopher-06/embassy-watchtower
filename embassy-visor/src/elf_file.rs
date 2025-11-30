use object::{Object, ObjectSymbol};
use std::collections::HashMap;

/// Return elf-file's address-to-symbol map
pub fn get_addr_map(file: object::File<'_>) -> HashMap<u64, String> {
    let mut addr_map: HashMap<u64, String> = HashMap::new();

    for symbol in file.symbols() {
        let addr = symbol.address();
        if addr == 0 {
            continue;
        }

        // Add symbol name if available
        if let Ok(name) = symbol.name() {
            if !name.is_empty() {
                let demangled = rustc_demangle::demangle(name).to_string();
                addr_map.insert(addr, demangled);

                // Reinsert to overwrite potential aliases
            }
        }
    }

    addr_map
}

/// Helper function to extract short name from full symbol name
pub fn try_extract_short_name(full_name: &str) -> &str {
    let pool_index = full_name.find("::POOL").unwrap_or(full_name.len());
    &full_name[0..pool_index]
}

//! Tauri host entry. The Builder chain registers Phase 1's two IPC commands
//! (read_exif_summary, list_fixtures). Phase 2/3/4 add to the handler list.
//!
//! D-08 invariant: Tauri-only code lives here. The four pfp-* lib crates carry
//! zero `tauri::*` symbols.

mod commands;
mod error;

pub fn run() {
    tauri::Builder::default()
        // Phase 2/3 will add:
        //   .register_asynchronous_uri_scheme_protocol("pfp-thumb", ...)
        //   .register_asynchronous_uri_scheme_protocol("pfp-tile", ...)
        // Hook point preserved here as a comment to make the extension obvious.
        .invoke_handler(tauri::generate_handler![
            commands::exif::read_exif_summary,
            commands::exif::list_fixtures,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

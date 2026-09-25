/// Disables Ubuntu's `appmenu-gtk-module`, which crashes druid's `MenuBar` with a stack
/// overflow when the global menu proxy tries to mirror it. Setting these in `.cargo/config.toml`
/// only affects `cargo run`/`cargo build`, not the binary once it is packaged and installed
/// (`.deb`/`.rpm`), so the same override is applied here at process startup, before GTK reads
/// them.
pub fn disable_ubuntu_global_menu() {
    unsafe {
        std::env::set_var("GTK_MODULES", "");
        std::env::set_var("UBUNTU_MENUPROXY", "0");
    }
}

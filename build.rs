fn main() {
    #[cfg(windows)]
    embed_windows_icon();
}

#[cfg(windows)]
fn embed_windows_icon() {
    if let Err(err) = winresource::WindowsResource::new()
        .set_icon("resources/images/icons/fast-diary.ico")
        .compile()
    {
        eprintln!("failed to embed the Windows executable icon: {err}");
        std::process::exit(1);
    }
}

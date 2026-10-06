fn main() {
    #[cfg(not(debug_assertions))]
    let _ = fix_path_env::fix();
    ley_desktop_lib::run();
}

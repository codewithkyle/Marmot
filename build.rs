fn main() {
    pkg_config::probe_library("fontconfig").expect("fontconfig development files are required");
}

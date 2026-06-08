fn main() {
    println!("cargo:rerun-if-changed=../../migrations/platform");
    println!("cargo:rerun-if-changed=../../migrations/agency");
}

fn main() {
    let h = std::env::args().nth(1).unwrap();
    let bytes = hex::decode(&h).unwrap();
    println!("{}", ergo_sandbox::decompile::decompile_bytes(&bytes).unwrap());
}

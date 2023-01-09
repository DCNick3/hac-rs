use hac::crypto::keyset::KeySet;

fn main() {
    let keyset = KeySet::from_system(None).unwrap();

    println!("Hello, world!");
}

pub use gix::features::hash::Sha1;
use hex::encode;

pub fn sha1_hexdigest(input: &[u8]) -> String {
    let mut h = Sha1::default();
    h.update(input);
    encode(h.digest())
}

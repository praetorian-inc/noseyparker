#[derive(Default)]
pub struct Sha1(gix::features::hash::Sha1);

pub type Sha1Digest = [u8; 20];

impl Sha1 {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, input: &[u8]) {
        self.0.update(input);
    }

    pub fn hexdigest(self) -> String {
        hex::encode(self.0.digest())
    }

    pub fn digest(self) -> Sha1Digest {
        self.0.digest()
    }
}

pub fn sha1_hexdigest(input: &[u8]) -> String {
    let mut h = Sha1::default();
    h.update(input);
    h.hexdigest()
}

// XXX implement a Write instance for `Sha1`, in an attempt to avoid allocations for
// formatting the input length. Not sure how well this actually avoids allocation.
impl std::io::Write for Sha1 {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.update(buf);
        Ok(buf.len())
    }

    #[inline]
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn empty() {
        assert_eq!(sha1_hexdigest(&[]), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
    }
}

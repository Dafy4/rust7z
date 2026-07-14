use super::Compressor;

/// RLE simple : chaque paire d'octets en sortie = (compteur, valeur).
/// compteur est dans [1, 255]. Une répétition plus longue est simplement
/// découpée en plusieurs paires.
///
/// C'est le plus simple des algos vus en cours : bon pour des données très
/// répétitives (ex: image avec de grands aplats de couleur), mais peut
/// doubler la taille sur des données aléatoires (pas de compression garantie).
pub struct Rle;

impl Compressor for Rle {
    fn name(&self) -> &'static str {
        "RLE"
    }

    fn compress(&self, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        if data.is_empty() {
            return out;
        }

        let mut i = 0;
        while i < data.len() {
            let current = data[i];
            let mut run_len: u32 = 1;

            while i + (run_len as usize) < data.len()
                && data[i + run_len as usize] == current
                && run_len < 255
            {
                run_len += 1;
            }

            out.push(run_len as u8);
            out.push(current);

            i += run_len as usize;
        }

        out
    }

    fn decompress(&self, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut i = 0;
        while i + 1 < data.len() {
            let count = data[i];
            let value = data[i + 1];
            for _ in 0..count {
                out.push(value);
            }
            i += 2;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let rle = Rle;
        let original = b"aaaabbbcccccccccccccccddddd".to_vec();
        let compressed = rle.compress(&original);
        let decompressed = rle.decompress(&compressed);
        assert_eq!(original, decompressed);
    }

    #[test]
    fn empty() {
        let rle = Rle;
        assert_eq!(rle.compress(&[]), Vec::<u8>::new());
        assert_eq!(rle.decompress(&[]), Vec::<u8>::new());
    }

    #[test]
    fn long_run_over_255() {
        let rle = Rle;
        let original = vec![b'x'; 600];
        let compressed = rle.compress(&original);
        let decompressed = rle.decompress(&compressed);
        assert_eq!(original, decompressed);
    }
}
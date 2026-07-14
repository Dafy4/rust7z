use super::Compressor;
use std::collections::HashMap;

/// LZW : construit un dictionnaire de motifs à la volée.
/// - Dictionnaire initial : les 256 octets seuls (codes 0..255).
/// - Chaque code est encodé sur 16 bits (u16, big-endian) : plus simple et
///   plus robuste à coder qu'un encodage à largeur variable, au prix d'un
///   peu moins de compression sur de très petits fichiers.
/// - Le dictionnaire est plafonné à 65536 entrées (limite du u16) ; une fois
///   plein, on arrête d'ajouter de nouvelles entrées mais on continue de
///   compresser avec le dictionnaire existant (pas de reset, par simplicité).
pub struct Lzw;

const MAX_DICT_SIZE: usize = 1 << 16; // 65536

impl Compressor for Lzw {
    fn name(&self) -> &'static str {
        "LZW"
    }

    fn compress(&self, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        if data.is_empty() {
            return out;
        }

        // Dictionnaire : motif (Vec<u8>) -> code
        let mut dictionary: HashMap<Vec<u8>, u16> = HashMap::new();
        for i in 0..256u16 {
            dictionary.insert(vec![i as u8], i);
        }
        let mut next_code: u32 = 256;

        let mut current: Vec<u8> = Vec::new();

        for &byte in data {
            let mut candidate = current.clone();
            candidate.push(byte);

            if dictionary.contains_key(&candidate) {
                current = candidate;
            } else {
                // On sort le code du plus long motif connu jusqu'ici.
                let code = dictionary[&current];
                out.extend_from_slice(&code.to_be_bytes());

                if next_code < MAX_DICT_SIZE as u32 {
                    dictionary.insert(candidate, next_code as u16);
                    next_code += 1;
                }

                current = vec![byte];
            }
        }

        if !current.is_empty() {
            let code = dictionary[&current];
            out.extend_from_slice(&code.to_be_bytes());
        }

        out
    }

    fn decompress(&self, data: &[u8]) -> Vec<u8> {
        if data.is_empty() {
            return Vec::new();
        }

        // Dictionnaire inverse : code -> motif
        let mut dictionary: Vec<Vec<u8>> = (0..256u32).map(|i| vec![i as u8]).collect();

        let mut out = Vec::new();
        let mut prev: Option<Vec<u8>> = None;

        let mut pos = 0;
        while pos + 1 < data.len() + 1 && pos + 2 <= data.len() {
            let code = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
            pos += 2;

            let entry: Vec<u8> = if code < dictionary.len() {
                dictionary[code].clone()
            } else if code == dictionary.len() {
                // Cas particulier classique du LZW (cWcWc) :
                // le code n'existe pas encore, il désigne "motif précédent + son propre premier octet".
                let mut e = prev.clone().expect("code invalide en début de flux");
                let first = e[0];
                e.push(first);
                e
            } else {
                panic!("Code LZW invalide : {code}");
            };

            out.extend_from_slice(&entry);

            if let Some(p) = &prev {
                if dictionary.len() < MAX_DICT_SIZE {
                    let mut new_entry = p.clone();
                    new_entry.push(entry[0]);
                    dictionary.push(new_entry);
                }
            }

            prev = Some(entry);
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_text() {
        let lzw = Lzw;
        let original = b"TOBEORNOTTOBEORTOBEORNOT".to_vec();
        let compressed = lzw.compress(&original);
        let decompressed = lzw.decompress(&compressed);
        assert_eq!(original, decompressed);
    }

    #[test]
    fn empty() {
        let lzw = Lzw;
        assert_eq!(lzw.compress(&[]), Vec::<u8>::new());
        assert_eq!(lzw.decompress(&[]), Vec::<u8>::new());
    }

    #[test]
    fn single_byte() {
        let lzw = Lzw;
        let original = vec![42u8];
        let compressed = lzw.compress(&original);
        let decompressed = lzw.decompress(&compressed);
        assert_eq!(original, decompressed);
    }

    #[test]
    fn roundtrip_binary() {
        let lzw = Lzw;
        let original: Vec<u8> = (0..=255).cycle().take(3000).collect();
        let compressed = lzw.compress(&original);
        let decompressed = lzw.decompress(&compressed);
        assert_eq!(original, decompressed);
    }

    #[test]
    fn roundtrip_highly_repetitive() {
        let lzw = Lzw;
        let original = vec![b'a'; 10000];
        let compressed = lzw.compress(&original);
        let decompressed = lzw.decompress(&compressed);
        assert_eq!(original, decompressed);
        assert!(compressed.len() < original.len());
    }
}
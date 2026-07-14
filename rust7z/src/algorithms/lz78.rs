use super::Compressor;
use std::collections::HashMap;

/// LZ78 : encode chaque nouveau motif comme un couple
/// (index du plus long préfixe déjà vu dans le dictionnaire, nouveau caractère).
///
/// Différence avec LZW : LZ78 transmet explicitement le caractère qui suit
/// le préfixe à chaque token, alors que LZW ne transmet que des indices
/// (le caractère est implicite, déduit du dictionnaire). LZ78 est donc
/// légèrement moins compact que LZW mais plus simple à décoder (pas de cas
/// particulier "cWcWc").
///
/// Format de token (4 octets, fixe) :
///   [u16 index_prefixe] [u8 has_char (0 ou 1)] [u8 caractère (0 si has_char=0)]
/// `has_char = 0` n'apparaît que sur le tout dernier token, quand le motif
/// restant en fin de fichier n'a pas de caractère suivant à coller dessus.
pub struct Lz78;

const MAX_DICT_SIZE: u32 = 65536; // limite du u16 (indices 0..65535)

impl Compressor for Lz78 {
    fn name(&self) -> &'static str {
        "LZ78"
    }

    fn compress(&self, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());

        if data.is_empty() {
            return out;
        }

        // dictionnaire : motif -> index (index 0 réservé au "préfixe vide")
        let mut dictionary: HashMap<Vec<u8>, u32> = HashMap::new();
        let mut next_index: u32 = 1;
        let mut current: Vec<u8> = Vec::new();

        for &byte in data {
            let mut candidate = current.clone();
            candidate.push(byte);

            if dictionary.contains_key(&candidate) {
                current = candidate;
            } else {
                let idx = if current.is_empty() {
                    0
                } else {
                    dictionary[&current]
                };

                out.extend_from_slice(&(idx as u16).to_be_bytes());
                out.push(1); // has_char
                out.push(byte);

                if next_index < MAX_DICT_SIZE {
                    dictionary.insert(candidate, next_index);
                    next_index += 1;
                }

                current = Vec::new();
            }
        }

        // Motif restant non "fermé" par un caractère suivant : token final spécial.
        if !current.is_empty() {
            let idx = dictionary[&current];
            out.extend_from_slice(&(idx as u16).to_be_bytes());
            out.push(0); // has_char = false
            out.push(0); // inutilisé
        }

        out
    }

    fn decompress(&self, data: &[u8]) -> Vec<u8> {
        if data.len() < 4 {
            return Vec::new();
        }
        let original_len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
        if original_len == 0 {
            return Vec::new();
        }

        // dictionnaire inverse : index -> motif (index 0 = motif vide)
        let mut dictionary: Vec<Vec<u8>> = vec![Vec::new()];
        let mut next_index: u32 = 1;

        let mut out = Vec::with_capacity(original_len);
        let mut pos = 4;

        while pos + 4 <= data.len() && out.len() < original_len {
            let idx = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
            let has_char = data[pos + 2];
            let char_value = data[pos + 3];
            pos += 4;

            let prefix = dictionary[idx].clone();

            if has_char == 1 {
                let mut entry = prefix;
                entry.push(char_value);
                out.extend_from_slice(&entry);

                if next_index < MAX_DICT_SIZE {
                    dictionary.push(entry);
                    next_index += 1;
                }
            } else {
                // Token final : on ne fait que restituer le préfixe, sans
                // ajouter de nouvelle entrée (comme à la compression).
                out.extend_from_slice(&prefix);
            }
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_text() {
        let lz78 = Lz78;
        let original = b"TOBEORNOTTOBEORTOBEORNOT".to_vec();
        let compressed = lz78.compress(&original);
        let decompressed = lz78.decompress(&compressed);
        assert_eq!(original, decompressed);
    }

    #[test]
    fn empty() {
        let lz78 = Lz78;
        let compressed = lz78.compress(&[]);
        let decompressed = lz78.decompress(&compressed);
        assert_eq!(decompressed, Vec::<u8>::new());
    }

    #[test]
    fn single_byte() {
        let lz78 = Lz78;
        let original = vec![42u8];
        let compressed = lz78.compress(&original);
        let decompressed = lz78.decompress(&compressed);
        assert_eq!(original, decompressed);
    }

    #[test]
    fn roundtrip_binary() {
        let lz78 = Lz78;
        let original: Vec<u8> = (0..=255).cycle().take(3000).collect();
        let compressed = lz78.compress(&original);
        let decompressed = lz78.decompress(&compressed);
        assert_eq!(original, decompressed);
    }

    #[test]
    fn roundtrip_highly_repetitive() {
        let lz78 = Lz78;
        let original = vec![b'a'; 10000];
        let compressed = lz78.compress(&original);
        let decompressed = lz78.decompress(&compressed);
        assert_eq!(original, decompressed);
        assert!(compressed.len() < original.len());
    }

    #[test]
    fn roundtrip_ends_mid_match() {
        // Force le cas où le tout dernier motif n'a pas de caractère suivant.
        let lz78 = Lz78;
        let original = b"abababab".to_vec();
        let compressed = lz78.compress(&original);
        let decompressed = lz78.decompress(&compressed);
        assert_eq!(original, decompressed);
    }
}
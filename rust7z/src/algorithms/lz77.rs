use super::Compressor;
use std::collections::HashMap;

/// LZ77 : recherche, dans une fenêtre glissante des octets déjà lus, la plus
/// longue correspondance avec ce qui arrive, et la remplace par une
/// référence (distance, longueur) au lieu de recopier les octets en clair.
/// C'est la base historique du DEFLATE (donc du vrai .zip / .gz).
///
/// Format des tokens (largeur variable, contrairement à LZ78) :
///   - `0x00` [octet]                                 -> littéral
///   - `0x01` [u16 distance][u8 longueur][octet]      -> correspondance suivie d'un littéral
///   - `0x02` [u16 distance][u8 longueur]              -> correspondance qui va jusqu'à la fin du fichier (pas de littéral suivant)
///
/// La recherche de correspondances utilise une table de hachage sur des
/// préfixes de 3 octets (comme zlib) pour éviter un scan linéaire de toute
/// la fenêtre à chaque position : complexité pratique quasi-linéaire au lieu
/// de O(n × fenêtre).
pub struct Lz77;

const WINDOW_SIZE: usize = 4096;
const MAX_MATCH_LEN: usize = 255;
const MIN_MATCH_LEN: usize = 3;
const MAX_CANDIDATES: usize = 32; // nb d'occurrences récentes gardées par préfixe

fn key3(data: &[u8], pos: usize) -> Option<[u8; 3]> {
    if pos + 3 <= data.len() {
        Some([data[pos], data[pos + 1], data[pos + 2]])
    } else {
        None
    }
}

impl Compressor for Lz77 {
    fn name(&self) -> &'static str {
        "LZ77"
    }

    fn compress(&self, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());

        let mut table: HashMap<[u8; 3], Vec<usize>> = HashMap::new();
        let mut pos = 0;

        while pos < data.len() {
            let max_len = (data.len() - pos).min(MAX_MATCH_LEN);
            let mut best_len = 0usize;
            let mut best_offset = 0usize;

            if max_len >= MIN_MATCH_LEN {
                if let Some(k) = key3(data, pos) {
                    if let Some(candidates) = table.get(&k) {
                        for &start in candidates.iter().rev() {
                            if pos - start > WINDOW_SIZE {
                                continue;
                            }
                            let mut len = 0;
                            while len < max_len && data[start + len] == data[pos + len] {
                                len += 1;
                            }
                            if len > best_len {
                                best_len = len;
                                best_offset = pos - start;
                                if len == max_len {
                                    break; // on ne peut pas trouver mieux
                                }
                            }
                        }
                    }
                }
            }

            // On enregistre la position courante pour les recherches futures.
            if let Some(k) = key3(data, pos) {
                let entry = table.entry(k).or_default();
                entry.push(pos);
                if entry.len() > MAX_CANDIDATES {
                    entry.remove(0);
                }
            }

            if best_len >= MIN_MATCH_LEN {
                let next_pos = pos + best_len;
                if next_pos < data.len() {
                    out.push(1);
                    out.extend_from_slice(&(best_offset as u16).to_be_bytes());
                    out.push(best_len as u8);
                    out.push(data[next_pos]);
                    pos = next_pos + 1;
                } else {
                    out.push(2);
                    out.extend_from_slice(&(best_offset as u16).to_be_bytes());
                    out.push(best_len as u8);
                    pos = next_pos;
                }
            } else {
                out.push(0);
                out.push(data[pos]);
                pos += 1;
            }
        }

        out
    }

    fn decompress(&self, data: &[u8]) -> Vec<u8> {
        if data.len() < 4 {
            return Vec::new();
        }
        let original_len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;

        let mut out = Vec::with_capacity(original_len);
        let mut pos = 4;

        while out.len() < original_len && pos < data.len() {
            let token_type = data[pos];
            pos += 1;

            match token_type {
                0 => {
                    out.push(data[pos]);
                    pos += 1;
                }
                1 => {
                    let offset = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
                    pos += 2;
                    let length = data[pos] as usize;
                    pos += 1;
                    let next_char = data[pos];
                    pos += 1;

                    let start = out.len() - offset;
                    for i in 0..length {
                        let b = out[start + i]; // copie octet par octet : gère les chevauchements
                        out.push(b);
                    }
                    out.push(next_char);
                }
                2 => {
                    let offset = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
                    pos += 2;
                    let length = data[pos] as usize;
                    pos += 1;

                    let start = out.len() - offset;
                    for i in 0..length {
                        let b = out[start + i];
                        out.push(b);
                    }
                }
                _ => panic!("Token LZ77 invalide : {token_type}"),
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
        let lz77 = Lz77;
        let original = b"the quick brown fox the quick brown fox jumps".to_vec();
        let compressed = lz77.compress(&original);
        let decompressed = lz77.decompress(&compressed);
        assert_eq!(original, decompressed);
    }

    #[test]
    fn empty() {
        let lz77 = Lz77;
        let compressed = lz77.compress(&[]);
        let decompressed = lz77.decompress(&compressed);
        assert_eq!(decompressed, Vec::<u8>::new());
    }

    #[test]
    fn single_byte() {
        let lz77 = Lz77;
        let original = vec![7u8];
        let compressed = lz77.compress(&original);
        let decompressed = lz77.decompress(&compressed);
        assert_eq!(original, decompressed);
    }

    #[test]
    fn roundtrip_overlapping_match() {
        // "aaaaaaaaaa" force un match dont la longueur dépasse la distance
        // (chevauchement), cas classique et piégeux du LZ77.
        let lz77 = Lz77;
        let original = vec![b'a'; 500];
        let compressed = lz77.compress(&original);
        let decompressed = lz77.decompress(&compressed);
        assert_eq!(original, decompressed);
        assert!(compressed.len() < original.len());
    }

    #[test]
    fn roundtrip_binary() {
        let lz77 = Lz77;
        let original: Vec<u8> = (0..=255).cycle().take(3000).collect();
        let compressed = lz77.compress(&original);
        let decompressed = lz77.decompress(&compressed);
        assert_eq!(original, decompressed);
    }

    #[test]
    fn roundtrip_no_repetition() {
        // Données sans aucune répétition : que des littéraux.
        let lz77 = Lz77;
        let original: Vec<u8> = (0..=200).collect();
        let compressed = lz77.compress(&original);
        let decompressed = lz77.decompress(&compressed);
        assert_eq!(original, decompressed);
    }
}
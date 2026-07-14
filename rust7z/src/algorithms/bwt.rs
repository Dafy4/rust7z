use super::huffman::Huffman;
use super::rle::Rle;
use super::Compressor;

/// Taille de bloc pour la transformée de Burrows-Wheeler.
///
/// BWT nécessite de trier toutes les rotations cycliques du bloc : notre
/// implémentation naïve est en O(n² log n) dans le pire cas (en pratique
/// bien plus rapide grâce à la comparaison de tranches optimisée par Rust).
/// Les vrais outils (bzip2) utilisent un tableau de suffixes en O(n log n),
/// plus complexe à implémenter ; on traite donc les données par blocs de
/// taille raisonnable pour garder un temps de calcul acceptable.
const BLOCK_SIZE: usize = 4096;

/// Transformée de Burrows-Wheeler sur un seul bloc.
/// Retourne (dernière_colonne, index_de_la_chaîne_originale_dans_le_tri).
///
/// Astuce d'implémentation : au lieu de comparer des rotations avec une
/// indexation modulo (lente), on double le bloc (`block ++ block`) et on
/// compare directement des tranches contiguës — bien plus rapide.
fn bwt_transform_block(block: &[u8]) -> (Vec<u8>, u32) {
    let n = block.len();
    if n == 0 {
        return (Vec::new(), 0);
    }

    let mut doubled = Vec::with_capacity(2 * n);
    doubled.extend_from_slice(block);
    doubled.extend_from_slice(block);

    let mut rotations: Vec<usize> = (0..n).collect();
    rotations.sort_by(|&a, &b| doubled[a..a + n].cmp(&doubled[b..b + n]));

    let mut output = vec![0u8; n];
    let mut original_index = 0u32;
    for (row, &start) in rotations.iter().enumerate() {
        output[row] = doubled[start + n - 1];
        if start == 0 {
            original_index = row as u32;
        }
    }

    (output, original_index)
}

/// Transformée inverse : reconstruit le bloc original à partir de la
/// dernière colonne (`bwt`) et de l'index de la chaîne originale.
/// Algorithme standard du "LF-mapping" (utilisé par bzip2).
fn bwt_inverse_block(bwt: &[u8], original_index: u32) -> Vec<u8> {
    let n = bwt.len();
    if n == 0 {
        return Vec::new();
    }

    let mut count = [0usize; 256];
    for &b in bwt {
        count[b as usize] += 1;
    }
    let mut starts = [0usize; 256];
    let mut sum = 0;
    for c in 0..256 {
        starts[c] = sum;
        sum += count[c];
    }

    let mut next = vec![0usize; n];
    let mut occurrence = [0usize; 256];
    for (i, &b) in bwt.iter().enumerate() {
        let c = b as usize;
        next[i] = starts[c] + occurrence[c];
        occurrence[c] += 1;
    }

    let mut result = vec![0u8; n];
    let mut row = original_index as usize;
    for i in (0..n).rev() {
        result[i] = bwt[row];
        row = next[row];
    }

    result
}

/// Move-To-Front : remplace chaque octet par sa position dans une liste des
/// 256 octets, puis remonte l'octet utilisé en tête de liste. Regroupe les
/// répétitions locales (fréquentes après un BWT) en suites de petits nombres
/// (souvent des 0), ce qui prépare idéalement le terrain pour RLE + Huffman.
fn mtf_encode(data: &[u8]) -> Vec<u8> {
    let mut table: Vec<u8> = (0..=255u8).collect();
    let mut out = Vec::with_capacity(data.len());

    for &b in data {
        let pos = table.iter().position(|&x| x == b).unwrap();
        out.push(pos as u8);
        table.remove(pos);
        table.insert(0, b);
    }

    out
}

fn mtf_decode(data: &[u8]) -> Vec<u8> {
    let mut table: Vec<u8> = (0..=255u8).collect();
    let mut out = Vec::with_capacity(data.len());

    for &pos in data {
        let b = table[pos as usize];
        out.push(b);
        table.remove(pos as usize);
        table.insert(0, b);
    }

    out
}

/// Pipeline complet façon bzip2 : BWT -> MTF -> RLE -> Huffman.
/// BWT seul ne compresse rien ; c'est la combinaison des quatre étapes qui
/// donne un vrai gain, chacune exploitant une redondance différente :
///   - BWT   : regroupe les caractères similaires
///   - MTF   : transforme ces regroupements en suites de petits nombres
///   - RLE   : écrase les suites répétées (souvent des 0) créées par MTF
///   - Huffman : encode le résultat selon la fréquence des symboles
pub struct Bwt;

impl Compressor for Bwt {
    fn name(&self) -> &'static str {
        "BWT+MTF+RLE+Huffman"
    }

    fn compress(&self, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());

        if data.is_empty() {
            out.extend_from_slice(&0u32.to_be_bytes()); // num_blocks = 0
            return out;
        }

        let blocks: Vec<&[u8]> = data.chunks(BLOCK_SIZE).collect();
        out.extend_from_slice(&(blocks.len() as u32).to_be_bytes());

        let mut concatenated_bwt = Vec::with_capacity(data.len());

        for block in &blocks {
            let (bwt_bytes, original_index) = bwt_transform_block(block);
            out.extend_from_slice(&original_index.to_be_bytes());
            out.extend_from_slice(&(block.len() as u32).to_be_bytes());
            concatenated_bwt.extend_from_slice(&bwt_bytes);
        }

        let mtf_bytes = mtf_encode(&concatenated_bwt);
        let rle_bytes = Rle.compress(&mtf_bytes);
        let huffman_bytes = Huffman.compress(&rle_bytes);

        out.extend_from_slice(&huffman_bytes);
        out
    }

    fn decompress(&self, data: &[u8]) -> Vec<u8> {
        if data.len() < 8 {
            return Vec::new();
        }

        let original_len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let num_blocks = u32::from_be_bytes([data[4], data[5], data[6], data[7]]) as usize;

        if original_len == 0 {
            return Vec::new();
        }

        let mut pos = 8;
        let mut block_infos = Vec::with_capacity(num_blocks);
        for _ in 0..num_blocks {
            let original_index = u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
            let block_len = u32::from_be_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]]) as usize;
            block_infos.push((original_index, block_len));
            pos += 8;
        }

        let huffman_payload = &data[pos..];
        let rle_bytes = Huffman.decompress(huffman_payload);
        let mtf_bytes = Rle.decompress(&rle_bytes);
        let concatenated_bwt = mtf_decode(&mtf_bytes);

        let mut out = Vec::with_capacity(original_len);
        let mut cursor = 0;
        for (original_index, block_len) in block_infos {
            let block_bwt = &concatenated_bwt[cursor..cursor + block_len];
            let plaintext_block = bwt_inverse_block(block_bwt, original_index);
            out.extend_from_slice(&plaintext_block);
            cursor += block_len;
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mtf_roundtrip() {
        let original = vec![3u8, 3, 1, 2, 3, 1, 1, 200, 200];
        let encoded = mtf_encode(&original);
        let decoded = mtf_decode(&encoded);
        assert_eq!(original, decoded);
    }

    #[test]
    fn bwt_single_block_roundtrip() {
        let block = b"banana".to_vec();
        let (bwt, idx) = bwt_transform_block(&block);
        let restored = bwt_inverse_block(&bwt, idx);
        assert_eq!(block, restored);
    }

    #[test]
    fn full_pipeline_roundtrip_text() {
        let bwt = Bwt;
        let original = b"abracadabra abracadabra abracadabra".to_vec();
        let compressed = bwt.compress(&original);
        let decompressed = bwt.decompress(&compressed);
        assert_eq!(original, decompressed);
    }

    #[test]
    fn full_pipeline_empty() {
        let bwt = Bwt;
        let compressed = bwt.compress(&[]);
        let decompressed = bwt.decompress(&compressed);
        assert_eq!(decompressed, Vec::<u8>::new());
    }

    #[test]
    fn full_pipeline_multi_block() {
        // Force plusieurs blocs (> BLOCK_SIZE octets).
        let bwt = Bwt;
        let original: Vec<u8> = (0..=255u8).cycle().take(BLOCK_SIZE * 3 + 100).collect();
        let compressed = bwt.compress(&original);
        let decompressed = bwt.decompress(&compressed);
        assert_eq!(original, decompressed);
    }

    #[test]
    fn full_pipeline_highly_repetitive() {
        let bwt = Bwt;
        let original = vec![b'x'; 5000];
        let compressed = bwt.compress(&original);
        let decompressed = bwt.decompress(&compressed);
        assert_eq!(original, decompressed);
        assert!(compressed.len() < original.len());
    }
}
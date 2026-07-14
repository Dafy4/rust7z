pub mod rle;
pub mod huffman;
pub mod lzw;
pub mod lz78;
pub mod lz77;
pub mod bwt;

/// Interface commune que chaque algorithme de compression doit implémenter.
/// Travaille toujours sur des octets bruts (`&[u8]`) : c'est ce qui permet
/// de compresser texte, image, audio ou vidéo avec le même pipeline,
/// puisqu'au niveau binaire ce sont tous des flux d'octets.
pub trait Compressor {
    fn compress(&self, data: &[u8]) -> Vec<u8>;
    fn decompress(&self, data: &[u8]) -> Vec<u8>;
    fn name(&self) -> &'static str;
}

/// Identifiants d'algorithme stockés dans l'en-tête du fichier archive,
/// pour que la décompression sache automatiquement quel algo utiliser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlgoId {
    Rle = 0,
    Huffman = 1,
    Lzw = 2,
    Lz78 = 3,
    Lz77 = 4,
    Bwt = 5,
}

impl AlgoId {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(AlgoId::Rle),
            1 => Some(AlgoId::Huffman),
            2 => Some(AlgoId::Lzw),
            3 => Some(AlgoId::Lz78),
            4 => Some(AlgoId::Lz77),
            5 => Some(AlgoId::Bwt),
            _ => None,
        }
    }
}
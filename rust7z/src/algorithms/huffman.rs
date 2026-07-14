use super::Compressor;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::collections::HashMap;

/// Arbre de Huffman minimal : feuille (octet) ou noeud interne (deux enfants).
#[derive(Debug, Clone)]
enum Node {
    Leaf(u8),
    Internal(Box<Node>, Box<Node>),
}

/// Élément du tas binaire (min-heap sur la fréquence).
struct HeapItem {
    freq: u64,
    // ordre d'insertion : sert de tie-break déterministe pour que
    // compress() et decompress() reconstruisent EXACTEMENT le même arbre
    // à partir des mêmes fréquences (important, sinon les codes ne
    // correspondent pas).
    order: usize,
    node: Node,
}

impl PartialEq for HeapItem {
    fn eq(&self, other: &Self) -> bool {
        self.freq == other.freq && self.order == other.order
    }
}
impl Eq for HeapItem {}
impl Ord for HeapItem {
    fn cmp(&self, other: &Self) -> Ordering {
        // BinaryHeap est un max-heap : on inverse pour avoir un min-heap.
        other
            .freq
            .cmp(&self.freq)
            .then_with(|| other.order.cmp(&self.order))
    }
}
impl PartialOrd for HeapItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct Huffman;

fn build_frequencies(data: &[u8]) -> HashMap<u8, u64> {
    let mut freqs: HashMap<u8, u64> = HashMap::new();
    for &b in data {
        *freqs.entry(b).or_insert(0) += 1;
    }
    freqs
}

fn build_tree(freqs: &HashMap<u8, u64>) -> Option<Node> {
    if freqs.is_empty() {
        return None;
    }

    let mut heap = BinaryHeap::new();
    // Tri des symboles par valeur d'octet pour un ordre déterministe,
    // indépendant de l'itération (non ordonnée) de la HashMap.
    let mut sorted: Vec<(&u8, &u64)> = freqs.iter().collect();
    sorted.sort_by_key(|(b, _)| **b);

    let mut order = 0usize;
    for pair in sorted {
        let byte = *pair.0;
        let freq = *pair.1;
        heap.push(HeapItem {
            freq,
            order,
            node: Node::Leaf(byte),
        });
        order += 1;
    }

    if heap.len() == 1 {
        // Un seul symbole distinct : on l'enveloppe quand même dans un
        // noeud interne pour qu'il ait un code de longueur >= 1.
        let only = heap.pop().unwrap();
        let wrapped = Node::Internal(
            Box::new(only.node),
            Box::new(Node::Leaf(0)), // feuille factice, jamais utilisée en pratique
        );
        return Some(wrapped);
    }

    while heap.len() > 1 {
        let a = heap.pop().unwrap();
        let b = heap.pop().unwrap();
        let merged = HeapItem {
            freq: a.freq + b.freq,
            order,
            node: Node::Internal(Box::new(a.node), Box::new(b.node)),
        };
        heap.push(merged);
        order += 1;
    }

    heap.pop().map(|item| item.node)
}

fn build_codes(node: &Node, prefix: Vec<bool>, codes: &mut HashMap<u8, Vec<bool>>) {
    match node {
        Node::Leaf(byte) => {
            let code = if prefix.is_empty() {
                vec![false]
            } else {
                prefix
            };
            codes.insert(*byte, code);
        }
        Node::Internal(left, right) => {
            let mut left_prefix = prefix.clone();
            left_prefix.push(false);
            build_codes(left, left_prefix, codes);

            let mut right_prefix = prefix;
            right_prefix.push(true);
            build_codes(right, right_prefix, codes);
        }
    }
}

fn write_bits(out: &mut Vec<u8>, bit_buf: &mut u8, bit_count: &mut u8, bit: bool) {
    if bit {
        *bit_buf |= 1 << (7 - *bit_count);
    }
    *bit_count += 1;
    if *bit_count == 8 {
        out.push(*bit_buf);
        *bit_buf = 0;
        *bit_count = 0;
    }
}

impl Compressor for Huffman {
    fn name(&self) -> &'static str {
        "Huffman"
    }

    fn compress(&self, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();

        let freqs = build_frequencies(data);

        // --- En-tête ---
        // [u32 original_len] [u16 symbol_count] puis pour chaque symbole: [u8 byte][u32 freq]
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(&(freqs.len() as u16).to_be_bytes());

        let mut sorted: Vec<(&u8, &u64)> = freqs.iter().collect();
        sorted.sort_by_key(|(b, _)| **b);
        for pair in &sorted {
            let byte = *pair.0;
            let freq = *pair.1;
            out.push(byte);
            out.extend_from_slice(&(freq as u32).to_be_bytes());
        }

        if data.is_empty() {
            return out;
        }

        let tree = build_tree(&freqs).unwrap();
        let mut codes = HashMap::new();
        build_codes(&tree, Vec::new(), &mut codes);

        // --- Corps : flux de bits compacté ---
        let mut bit_buf: u8 = 0;
        let mut bit_count: u8 = 0;
        for &byte in data {
            let code = &codes[&byte];
            for &bit in code {
                write_bits(&mut out, &mut bit_buf, &mut bit_count, bit);
            }
        }
        if bit_count > 0 {
            out.push(bit_buf); // dernier octet partiellement rempli
        }

        out
    }

    fn decompress(&self, data: &[u8]) -> Vec<u8> {
        if data.len() < 6 {
            return Vec::new();
        }
        let original_len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let symbol_count = u16::from_be_bytes([data[4], data[5]]) as usize;

        if original_len == 0 {
            return Vec::new();
        }

        let mut freqs = HashMap::new();
        let mut pos = 6;
        for _ in 0..symbol_count {
            let byte = data[pos];
            let freq = u32::from_be_bytes([data[pos + 1], data[pos + 2], data[pos + 3], data[pos + 4]]) as u64;
            freqs.insert(byte, freq);
            pos += 5;
        }

        let tree = build_tree(&freqs).unwrap();

        let mut out = Vec::with_capacity(original_len);
        let mut node = &tree;

        'outer: for &byte in &data[pos..] {
            for i in 0..8 {
                let bit = (byte >> (7 - i)) & 1 == 1;
                node = match node {
                    Node::Internal(left, right) => {
                        if bit {
                            right
                        } else {
                            left
                        }
                    }
                    Node::Leaf(_) => unreachable!(),
                };
                if let Node::Leaf(b) = node {
                    out.push(*b);
                    node = &tree;
                    if out.len() == original_len {
                        break 'outer;
                    }
                }
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
        let h = Huffman;
        let original = b"abracadabra abracadabra".to_vec();
        let compressed = h.compress(&original);
        let decompressed = h.decompress(&compressed);
        assert_eq!(original, decompressed);
    }

    #[test]
    fn roundtrip_single_symbol() {
        let h = Huffman;
        let original = vec![b'z'; 50];
        let compressed = h.compress(&original);
        let decompressed = h.decompress(&compressed);
        assert_eq!(original, decompressed);
    }

    #[test]
    fn empty() {
        let h = Huffman;
        let compressed = h.compress(&[]);
        let decompressed = h.decompress(&compressed);
        assert_eq!(decompressed, Vec::<u8>::new());
    }

    #[test]
    fn roundtrip_binary_data() {
        let h = Huffman;
        let original: Vec<u8> = (0..=255).cycle().take(2000).collect();
        let compressed = h.compress(&original);
        let decompressed = h.decompress(&compressed);
        assert_eq!(original, decompressed);
    }
}
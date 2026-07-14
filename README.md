# ANDRIAMAHARIVONISOA Aina Dafy Ange - GL M2 

# Rust7z - Compresseur de données

Rust7z est un compresseur/décompresseur de données universel, inspiré de 7‑Zip, entièrement écrit en Rust.
Il implémente une suite d’algorithmes de compression sans perte (RLE, Huffman, LZ77, LZ78, LZW, BWT) ainsi qu’une extension avec K‑means pour la compression avec perte sur les images, le tout couplé à un codage entropique (Huffman) pour un résultat optimal.

## Algorithmes disponibles
- RLE, Huffman, LZW, LZ78, LZ77, BWT, K-means (pour les images)

## Fonctionnalités 

-  6 algorithmes de compression sans perte : RLE, Huffman, LZ77, LZ78, LZW, BWT+MTF+RLE+Huffman.

- Mode automatique (--algo auto) : teste tous les algorithmes et conserve le meilleur résultat.

- Compression avec perte pour les images : utilise K‑means pour réduire le nombre de couleurs, suivi de Huffman.

- Support de nombreux formats d’images : BMP, PNG, JPEG, GIF, TIFF (via la crate image).

- Décompression automatique : détection du format original et restauration à l’identique.

- Interface en ligne de commande simple et ergonomique (crate clap).

- Cross‑compilation : produit des exécutables pour Linux et Windows.

- Extensions possibles : le code est modulaire pour ajouter facilement de nouveaux algorithmes.


## STRUCTURE DU PROJET

```txt
rust7z/
├── Cargo.toml
├── README.md
├── src/
│   ├── main.rs                # Point d’entrée, CLI, en‑tête de fichier
│   ├── image_utils.rs         # Chargement/sauvegarde d’images
│   ├── algorithms/
│   │   ├── mod.rs             # Définition du trait Compressor et AlgoId
│   │   ├── rle.rs
│   │   ├── huffman.rs
│   │   ├── lz77.rs
│   │   ├── lz78.rs
│   │   ├── lzw.rs
│   │   ├── bwt.rs
│   │   └── kmeans.rs          # Quantification par K‑means
│   └── bin/
│   |    └── gui.rs             # (optionnel) Interface graphique minimale
|   |──target/
|   |──test files/
|   |──compressed files/
|   |──decompressed files/
|
└── tests/                     # Fichiers de test (non inclus)
```

## Compilation 
- Prérequis : 
    * Rust et Cargo
 
```bash
cargo build --release
```

L'exécutable se trouve dans `target/release/rust7z`, pour windows : `target/x86_64-pc-windows-gnu/release/rust7z.exe`

## SYNTAXE GENERALE 

```bash
rust7z <COMMANDE> [OPTIONS] <INPUT> <OUTPUT>
```

Command :
- compress
- decompress

Options de compression :
* --algo <ALGO> : Algorithme : rle, huffman, lzw, lz78, lz77, bwt, auto (défaut : huffman)
* --lossy <0..10> : Niveau de perte pour les images (0 = sans perte, 10 = perte maximale)
* -h, --help : Affiche l’aide

## TEST EXECUTABLE LINUX

./target/release/rust7z compress entree.txt sortie.r7z --algo auto

## LANCER LE GUI 

./target/release/gui

## UTILISATION

# Compression
./target/release/rust7z compress fichier.txt fichier.r7z --algo huffman

# Compression avec perte (images)
./target/release/rust7z compress "tests files/video.mp4" "compressed files/video.r7z" --algo huffman --lossy 5

# Décompression
./target/release/rust7z decompress "compressed files/video.r7z" "decompressed files/video.mp4"
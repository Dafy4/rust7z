mod algorithms;
mod image_utils;

use algorithms::bwt::Bwt;
use algorithms::huffman::Huffman;
use algorithms::lz77::Lz77;
use algorithms::lz78::Lz78;
use algorithms::lzw::Lzw;
use algorithms::rle::Rle;
use algorithms::kmeans::quantize_pixels;
use algorithms::{AlgoId, Compressor};
use clap::{Parser, Subcommand};
use image::ImageFormat;
use image_utils::{load_image, save_image};
use std::fs;
use std::path::PathBuf;
use std::time::Instant;




const MAGIC: &[u8; 4] = b"R7Z1";


fn format_to_code(f: ImageFormat) -> u8 {
    match f {
        ImageFormat::Bmp => 1,
        ImageFormat::Png => 2,
        ImageFormat::Jpeg => 3,
        ImageFormat::Gif => 4,
        ImageFormat::Tiff => 5,
        _ => 0,
    }
}


fn code_to_format(c: u8) -> ImageFormat {
    match c {
        1 => ImageFormat::Bmp,
        2 => ImageFormat::Png,
        3 => ImageFormat::Jpeg,
        4 => ImageFormat::Gif,
        5 => ImageFormat::Tiff,
        _ => ImageFormat::Png,
    }
}

#[derive(Parser)]
#[command(name = "rust7z", about = "Un compresseur/décompresseur maison inspiré de 7-Zip")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compresser un fichier (texte, image, audio, vidéo... tout est binaire)
    Compress {
        input: PathBuf,
        output: PathBuf,
        #[arg(short, long, default_value = "huffman")]
        algo: String,
        #[arg(short, long, default_value_t = 0)]
        lossy: u8, // 0 = sans perte, 1-10 = niveau de perte
    },
    /// Décompresser un fichier .r7z (l'algo est détecté automatiquement)
    Decompress { input: PathBuf, output: PathBuf },
}

fn algo_from_name(name: &str) -> Option<(AlgoId, Box<dyn Compressor>)> {
    match name.to_lowercase().as_str() {
        "rle" => Some((AlgoId::Rle, Box::new(Rle))),
        "huffman" => Some((AlgoId::Huffman, Box::new(Huffman))),
        "lzw" => Some((AlgoId::Lzw, Box::new(Lzw))),
        "lz78" => Some((AlgoId::Lz78, Box::new(Lz78))),
        "lz77" => Some((AlgoId::Lz77, Box::new(Lz77))),
        "bwt" => Some((AlgoId::Bwt, Box::new(Bwt))),
        _ => None,
    }
}

fn algo_from_id(id: AlgoId) -> Box<dyn Compressor> {
    match id {
        AlgoId::Rle => Box::new(Rle),
        AlgoId::Huffman => Box::new(Huffman),
        AlgoId::Lzw => Box::new(Lzw),
        AlgoId::Lz78 => Box::new(Lz78),
        AlgoId::Lz77 => Box::new(Lz77),
        AlgoId::Bwt => Box::new(Bwt),
    }
}

/// Tous les algorithmes disponibles, utilisés par le mode `--algo auto`
/// pour tester chacun et garder automatiquement le meilleur résultat.
fn all_algos() -> Vec<(AlgoId, Box<dyn Compressor>)> {
    vec![
        (AlgoId::Rle, Box::new(Rle)),
        (AlgoId::Huffman, Box::new(Huffman)),
        (AlgoId::Lzw, Box::new(Lzw)),
        (AlgoId::Lz78, Box::new(Lz78)),
        (AlgoId::Lz77, Box::new(Lz77)),
        (AlgoId::Bwt, Box::new(Bwt)),
    ]
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        // Commands::Compress { input, output, algo, lossy } => {
        //     let data = fs::read(&input).expect("Impossible de lire le fichier d'entrée");
        //     let original_size = data.len();

        //     // 1. Prétraiter les données si lossy > 0
        //     let (preprocessed_data, preproc_params) = if lossy > 0 {
        //         // Détection de l'extension pour choisir le prétraitement
        //         let ext = input.extension().and_then(|s| s.to_str()).unwrap_or("");
        //         let preproc = match ext {
        //             "bmp" | "png" | "jpg" | "jpeg" => PreprocType::Quantize,
        //             "mp4" | "avi" | "mov" => PreprocType::SubSample,
        //             _ => PreprocType::None,
        //         };
        //         apply_preprocessing(&data, preproc, lossy)
        //     } else {
        //         (data.clone(), PreprocParams::none())
        //     };

        //     // 2. Sélectionner l'algorithme et compresser les données prétraitées
        //     let (algo_id, algo_display_name, file_out, elapsed) = if algo.to_lowercase() == "auto"
        //     {
        //         // Mode auto : on essaie tous les algos et on garde le plus petit résultat.
        //         println!("Mode auto : test de tous les algorithmes disponibles...\n");

        //         let mut best: Option<(AlgoId, &'static str, Vec<u8>)> = None;

        //         for (id, compressor) in all_algos() {
        //             let start = Instant::now();
        //             let payload = compressor.compress(&data);
        //             let elapsed = start.elapsed();

        //             let mut candidate = Vec::new();
        //             candidate.extend_from_slice(MAGIC);
        //             candidate.push(id as u8);
        //             candidate.extend_from_slice(&payload);

        //             println!(
        //                 "  {:<10} -> {:>10} octets ({:.2?})",
        //                 compressor.name(),
        //                 candidate.len(),
        //                 elapsed
        //             );

        //             let is_better = match &best {
        //                 None => true,
        //                 Some((_, _, best_bytes)) => candidate.len() < best_bytes.len(),
        //             };
        //             if is_better {
        //                 best = Some((id, compressor.name(), candidate));
        //             }
        //         }

        //         let (id, name, bytes) = best.expect("Aucun algorithme disponible");
        //         println!("\n=> Meilleur choix : {name}\n");
        //         (id, name, bytes, std::time::Duration::default())
        //     } else {
        //         let (id, compressor) = match algo_from_name(&algo) {
        //             Some(v) => v,
        //             None => {
        //                 eprintln!("Algorithme inconnu : {algo} (options: rle, huffman, lzw, lz78, lz77, bwt, auto)");
        //                 std::process::exit(1);
        //             }
        //         };

        //         let start = Instant::now();
        //         let payload = compressor.compress(&data);
        //         let elapsed = start.elapsed();

        //         let mut bytes = Vec::new();
        //         bytes.extend_from_slice(MAGIC);
        //         bytes.push(id as u8);
        //         bytes.extend_from_slice(&payload);

        //         (id, compressor.name(), bytes, elapsed)
        //     };
        //     let _ = algo_id; // déjà encodé dans file_out

        //     fs::write(&output, &file_out).expect("Impossible d'écrire le fichier de sortie");

        //     let compressed_size = file_out.len();
        //     let ratio = if original_size > 0 {
        //         100.0 * (1.0 - (compressed_size as f64 / original_size as f64))
        //     } else {
        //         0.0
        //     };

        //     println!("Algorithme       : {algo_display_name}");
        //     println!("Taille originale : {original_size} octets");
        //     println!("Taille compressée: {compressed_size} octets");
        //     println!("Gain             : {ratio:.2}%");
        //     if !elapsed.is_zero() {
        //         println!("Temps            : {elapsed:.2?}");
        //     }
        // }
        Commands::Compress { input, output, algo, lossy } => {
            let data = fs::read(&input).expect("Impossible de lire le fichier d'entrée");
            let original_size = data.len();

            // Appliquer K-means si lossy > 0 et si c'est une image
            let (processed_data, metadata) = if lossy > 0 {
                if let Some((pixels, width, height, format)) = load_image(&data) {
                    let k = (lossy as usize * 8 + 8).min(128).max(4);
                    let quantized = quantize_pixels(&pixels, k);
                    let new_data = save_image(&quantized, width, height, format);
                    let meta = Some((format, 0, 0));
                    (new_data, meta)
                } else {
                    // Pas une image : on garde les données brutes
                    (data, None)
                }
            } else {
                (data, None)
            };

            // Sélectionner l'algorithme
            let (algo_id, compressor) = match algo_from_name(&algo) {
                Some(v) => v,
                None => {
                    eprintln!("Algorithme inconnu : {algo}");
                    std::process::exit(1);
                }
            };

            let start = Instant::now();
            let payload = compressor.compress(&processed_data);
            let elapsed = start.elapsed();

            // Construire l'en-tête
            let mut bytes = Vec::new();
            bytes.extend_from_slice(MAGIC);
            bytes.push(algo_id as u8);

            let mut flags: u8 = 0;
            let mut meta_bytes = Vec::new();
            if let Some((format, width, height)) = metadata {
                flags |= 0b0000_0001;
                let format_code = match format {
                    image::ImageFormat::Bmp => 1,
                    image::ImageFormat::Png => 2,
                    image::ImageFormat::Jpeg => 3,
                    image::ImageFormat::Gif => 4,
                    image::ImageFormat::Tiff => 5,
                    _ => 0,
                };
                meta_bytes.push(format_code);
                // meta_bytes.extend_from_slice(&width.to_be_bytes());
                // meta_bytes.extend_from_slice(&height.to_be_bytes());
            }
            bytes.push(flags);
            bytes.extend_from_slice(&meta_bytes);
            bytes.extend_from_slice(&payload);

            fs::write(&output, &bytes).expect("Impossible d'écrire le fichier de sortie");

            let compressed_size = bytes.len();
            let ratio = if original_size > 0 {
                100.0 * (1.0 - (compressed_size as f64 / original_size as f64))
            } else { 0.0 };
            println!("Algorithme       : {}", compressor.name());
            println!("Taille originale : {original_size} octets");
            println!("Taille compressée: {compressed_size} octets");
            println!("Gain             : {ratio:.2}%");
            if !elapsed.is_zero() {
                println!("Temps            : {elapsed:.2?}");
            }
        }

        // Commands::Decompress { input, output } => {
        //     let file_in = fs::read(&input).expect("Impossible de lire le fichier d'entrée");

        //     if file_in.len() < 5 || &file_in[0..4] != MAGIC {
        //         eprintln!("Fichier invalide ou non reconnu (magic bytes manquants)");
        //         std::process::exit(1);
        //     }

        //     let algo_id = AlgoId::from_u8(file_in[4]).expect("Identifiant d'algorithme inconnu");
        //     let compressor = algo_from_id(algo_id);
        //     let payload = &file_in[5..];

        //     let start = Instant::now();
        //     let decompressed = compressor.decompress(payload);
        //     let elapsed = start.elapsed();

        //     fs::write(&output, &decompressed).expect("Impossible d'écrire le fichier de sortie");

        //     println!("Algorithme détecté : {}", compressor.name());
        //     println!("Taille restaurée   : {} octets", decompressed.len());
        //     println!("Temps              : {elapsed:.2?}");
        // }
       Commands::Decompress { input, output } => {
            let file_in = fs::read(&input).expect("Impossible de lire le fichier d'entrée");
            if file_in.len() < 6 || &file_in[0..4] != MAGIC {
                eprintln!("Fichier invalide");
                std::process::exit(1);
            }
            let algo_id = AlgoId::from_u8(file_in[4]).expect("Algo inconnu");
            let flags = file_in[5];
            let mut pos = 6;
            let mut has_image_format = false;
            let mut format_code = 0;
            if flags & 0b0000_0001 != 0 {
                has_image_format = true;
                format_code = file_in[pos];
                pos += 1;
            }
            let payload = &file_in[pos..];

            let compressor = algo_from_id(algo_id);
            let start = Instant::now();
            let decompressed = compressor.decompress(payload);
            let elapsed = start.elapsed();

            // Écrire les données décompressées directement
            fs::write(&output, &decompressed).expect("Impossible d'écrire le fichier de sortie");

            println!("Algorithme détecté : {}", compressor.name());
            println!("Taille restaurée   : {} octets", decompressed.len());
            if has_image_format {
                let format_name = match format_code {
                    1 => "BMP",
                    2 => "PNG",
                    3 => "JPEG",
                    4 => "GIF",
                    5 => "TIFF",
                    _ => "inconnu",
                };
                println!("Format d'image détecté : {}", format_name);
                // Optionnel : ajouter l'extension automatique si le nom de sortie n'en a pas
            }
            println!("Temps              : {elapsed:.2?}");
        }
    }
}
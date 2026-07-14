/// Quantifie les couleurs d'une image en utilisant K-means.
/// `pixels` : slice d'octets en format RVB (3 octets par pixel, ordre R,G,B).
/// `k` : nombre de clusters (couleurs finales).
/// Retourne un Vec<u8> des pixels quantifiés (même ordre et taille).
pub fn quantize_pixels(pixels: &[u8], k: usize) -> Vec<u8> {
    let n = pixels.len() / 3;
    if n == 0 || k == 0 || k >= n {
        return pixels.to_vec();
    }

    // Convertir en points (f64) pour K-means
    let mut points: Vec<[f64; 3]> = Vec::with_capacity(n);
    for chunk in pixels.chunks_exact(3) {
        points.push([
            chunk[0] as f64,
            chunk[1] as f64,
            chunk[2] as f64,
        ]);
    }

    // Initialisation des centroïdes : on prend les k premiers points
    let mut centroids: Vec<[f64; 3]> = points.iter().take(k).cloned().collect();

    // 20 itérations (suffisant pour une démo)
    for _ in 0..20 {
        // Assignation
        let mut clusters: Vec<Vec<usize>> = vec![Vec::new(); k];
        for (i, p) in points.iter().enumerate() {
            let mut best = 0;
            let mut best_dist = f64::MAX;
            for (j, c) in centroids.iter().enumerate() {
                let d = (p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2) + (p[2] - c[2]).powi(2);
                if d < best_dist {
                    best_dist = d;
                    best = j;
                }
            }
            clusters[best].push(i);
        }
        // Mise à jour des centroïdes
        let mut new_centroids = vec![[0.0, 0.0, 0.0]; k];
        for (j, cluster) in clusters.iter().enumerate() {
            if cluster.is_empty() {
                continue;
            }
            let mut sum = [0.0, 0.0, 0.0];
            for &idx in cluster {
                let p = points[idx];
                sum[0] += p[0];
                sum[1] += p[1];
                sum[2] += p[2];
            }
            let n_cluster = cluster.len() as f64;
            new_centroids[j] = [sum[0]/n_cluster, sum[1]/n_cluster, sum[2]/n_cluster];
        }
        centroids = new_centroids;
    }

    // Quantifier chaque pixel avec le centroïde le plus proche
    let mut quantized = Vec::with_capacity(points.len() * 3);
    for p in points {
        let mut best = 0;
        let mut best_dist = f64::MAX;
        for (j, c) in centroids.iter().enumerate() {
            let d = (p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2) + (p[2] - c[2]).powi(2);
            if d < best_dist {
                best_dist = d;
                best = j;
            }
        }
        let c = centroids[best];
        quantized.push(c[0] as u8);
        quantized.push(c[1] as u8);
        quantized.push(c[2] as u8);
    }
    quantized
}
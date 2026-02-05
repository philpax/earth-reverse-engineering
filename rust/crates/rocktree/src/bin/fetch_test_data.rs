//! Fetch raw protobuf data from Google Earth for test vector generation.
//!
//! This binary fetches planetoid, bulk, and node data and saves the raw
//! protobuf responses to disk for use in cross-implementation testing.

use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use rocktree::{BulkMetadata, BulkRequest, Client, NoCache, NodeRequest};

const OUTPUT_DIR: &str = "test_vectors";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create output directory.
    let output_path = Path::new(OUTPUT_DIR);
    fs::create_dir_all(output_path)?;

    println!("Fetching test data from Google Earth...\n");

    // Create client without cache.
    let client = Client::with_cache(NoCache);

    // Fetch planetoid and bulk metadata.
    let root_bulk = fetch_planetoid_and_bulk(&client, output_path).await?;

    // Fetch node data.
    fetch_nodes(&client, output_path, &root_bulk).await?;

    // Print summary.
    print_summary(output_path)?;

    Ok(())
}

async fn fetch_planetoid_and_bulk(
    client: &Client<NoCache>,
    output_path: &Path,
) -> Result<BulkMetadata, Box<dyn std::error::Error>> {
    // Fetch and save planetoid metadata.
    println!("1. Fetching planetoid metadata...");
    let planetoid_url = client.planetoid_url();
    let planetoid_bytes = client.fetch_bytes_from_url(&planetoid_url).await?;
    let planetoid_path = output_path.join("planetoid.pb");
    File::create(&planetoid_path)?.write_all(&planetoid_bytes)?;
    println!(
        "   Saved {} bytes to {}",
        planetoid_bytes.len(),
        planetoid_path.display()
    );

    // Decode to get epoch for next request.
    let planetoid = client.fetch_planetoid().await?;
    println!(
        "   Planetoid: radius={:.0}m, root_epoch={}",
        planetoid.radius, planetoid.root_epoch
    );

    // Fetch and save root bulk metadata.
    println!("\n2. Fetching root bulk metadata...");
    let bulk_request = BulkRequest::root(planetoid.root_epoch);
    let bulk_url = client.bulk_url(&bulk_request);
    let bulk_bytes = client.fetch_bytes_from_url(&bulk_url).await?;
    let bulk_path = output_path.join("bulk_root.pb");
    File::create(&bulk_path)?.write_all(&bulk_bytes)?;
    println!(
        "   Saved {} bytes to {}",
        bulk_bytes.len(),
        bulk_path.display()
    );

    // Decode to get node info.
    let root_bulk = client.fetch_bulk(&bulk_request).await?;
    println!(
        "   Root bulk: {} nodes, {} child bulks",
        root_bulk.nodes.len(),
        root_bulk.child_bulk_paths.len()
    );

    // Save bulk metadata as JSON for reference.
    let bulk_json_path = output_path.join("bulk_root.json");
    let bulk_json = serde_json::json!({
        "path": root_bulk.path,
        "head_node_center": [root_bulk.head_node_center.x, root_bulk.head_node_center.y, root_bulk.head_node_center.z],
        "meters_per_texel": root_bulk.meters_per_texel,
        "epoch": root_bulk.epoch,
        "node_count": root_bulk.nodes.len(),
        "child_bulk_paths": root_bulk.child_bulk_paths,
    });
    File::create(&bulk_json_path)?
        .write_all(serde_json::to_string_pretty(&bulk_json)?.as_bytes())?;

    Ok(root_bulk)
}

async fn fetch_nodes(
    client: &Client<NoCache>,
    output_path: &Path,
    root_bulk: &BulkMetadata,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n3. Fetching node data...");
    let mut nodes_fetched = 0;
    let max_nodes = 3;

    for node_meta in &root_bulk.nodes {
        if !node_meta.has_data {
            continue;
        }
        if nodes_fetched >= max_nodes {
            break;
        }

        let request = NodeRequest::new(
            node_meta.path.clone(),
            node_meta.epoch,
            node_meta.texture_format,
            node_meta.imagery_epoch,
        );

        let node_url = client.node_url(&request);
        let node_bytes = client.fetch_bytes_from_url(&node_url).await?;

        let safe_path = node_meta.path.replace('/', "_");
        let node_pb_path = output_path.join(format!("node_{safe_path}.pb"));
        File::create(&node_pb_path)?.write_all(&node_bytes)?;
        println!(
            "   Saved node '{}': {} bytes to {}",
            node_meta.path,
            node_bytes.len(),
            node_pb_path.display()
        );

        // Also decode and save summary.
        save_node_json(client, output_path, &request, &safe_path).await?;

        nodes_fetched += 1;
    }

    Ok(())
}

async fn save_node_json(
    client: &Client<NoCache>,
    output_path: &Path,
    request: &NodeRequest,
    safe_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let node = client.fetch_node(request).await?;
    let node_json_path = output_path.join(format!("node_{safe_path}.json"));
    let node_json = serde_json::json!({
        "path": node.path,
        "mesh_count": node.meshes.len(),
        "meshes": node.meshes.iter().enumerate().map(|(i, m)| {
            serde_json::json!({
                "index": i,
                "vertex_count": m.vertices.len(),
                "index_count": m.indices.len(),
                "texture_width": m.texture_width,
                "texture_height": m.texture_height,
                "uv_offset": [m.uv_transform.offset.x, m.uv_transform.offset.y],
                "uv_scale": [m.uv_transform.scale.x, m.uv_transform.scale.y],
                "first_vertices": m.vertices.iter().take(5).map(|v| {
                    serde_json::json!({
                        "x": v.x, "y": v.y, "z": v.z, "w": v.w, "u": v.u(), "v": v.v()
                    })
                }).collect::<Vec<_>>(),
                "first_indices": m.indices.iter().take(20).collect::<Vec<_>>(),
            })
        }).collect::<Vec<_>>(),
    });
    File::create(&node_json_path)?
        .write_all(serde_json::to_string_pretty(&node_json)?.as_bytes())?;
    Ok(())
}

fn print_summary(output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Summary ===");
    println!("Saved test vectors to '{OUTPUT_DIR}/':");
    println!("  - planetoid.pb: Raw planetoid metadata");
    println!("  - bulk_root.pb: Raw root bulk metadata");
    println!("  - bulk_root.json: Decoded bulk summary");
    for entry in fs::read_dir(output_path)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with("node_") && name_str.ends_with(".pb") {
            println!("  - {name_str}: Raw node data");
        }
    }

    println!("\nNext steps:");
    println!("1. Create C++ tool to decode these .pb files and output results");
    println!("2. Compare C++ output with Rust decoded JSON files");

    Ok(())
}

//! Compare C++ and Rust decoded test vectors to verify correctness.
//!
//! This is a turnkey solution that:
//! 1. Builds the C++ test vector decoder
//! 2. Runs it to produce `*_cpp.json` files
//! 3. Compares against the Rust `.json` files
//!
//! Run: `cargo run -p rocktree --features test-tools --bin compare_test_vectors -- <test_vectors_dir>`
//!
//! Prerequisites:
//! - Test vectors must already exist (`.pb` files from `fetch_test_data`)
//! - Rust JSON files must already exist (`.json` files from `fetch_test_data`)

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();
    let test_vectors_dir = args.get(1).map_or("test_vectors", String::as_str);

    // Determine paths relative to workspace root.
    let workspace_root = env::var("CARGO_MANIFEST_DIR").map_or_else(
        |_| Path::new(".").to_path_buf(),
        |p| {
            Path::new(&p)
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .to_path_buf()
        },
    );

    let client_dir = workspace_root.parent().unwrap().join("client");
    let test_vectors_path = workspace_root.join(test_vectors_dir);

    println!("Workspace root: {}", workspace_root.display());
    println!("Client dir: {}", client_dir.display());
    println!("Test vectors: {}\n", test_vectors_path.display());

    // Step 1: Build the C++ decoder.
    println!("=== Building C++ decoder ===");
    if let Err(e) = build_cpp_decoder(&client_dir) {
        eprintln!("Failed to build C++ decoder: {e}");
        std::process::exit(1);
    }
    println!("Build successful\n");

    // Step 2: Run the C++ decoder.
    println!("=== Running C++ decoder ===");
    if let Err(e) = run_cpp_decoder(&client_dir, &test_vectors_path) {
        eprintln!("Failed to run C++ decoder: {e}");
        std::process::exit(1);
    }
    println!("C++ decoding complete\n");

    // Step 3: Compare outputs.
    println!("=== Comparing outputs ===\n");
    let test_vectors_str = test_vectors_path.to_string_lossy();
    let mut all_passed = true;

    // Compare bulk metadata.
    println!("--- Bulk Metadata ---");
    if let Err(e) = compare_bulk_metadata(&test_vectors_str) {
        println!("FAILED: {e}");
        all_passed = false;
    } else {
        println!("PASSED\n");
    }

    // Compare node data.
    for node in &["024", "03", "134"] {
        println!("--- Node {node} ---");
        if let Err(e) = compare_node_data(&test_vectors_str, node) {
            println!("FAILED: {e}");
            all_passed = false;
        } else {
            println!("PASSED\n");
        }
    }

    if all_passed {
        println!("All comparisons PASSED!");
    } else {
        println!("Some comparisons FAILED!");
        std::process::exit(1);
    }
}

fn build_cpp_decoder(client_dir: &Path) -> Result<(), String> {
    // Try nix-shell first, fall back to direct g++.
    let nix_shell_path = client_dir.join("shell.nix");
    let use_nix = nix_shell_path.exists();

    let build_cmd = "g++ -std=c++17 -I. -Ieigen decode_test_vectors.cpp proto/rocktree.pb.cc $(pkg-config --cflags --libs protobuf) -o decode_test_vectors";

    let output = if use_nix {
        println!("  Using nix-shell for build...");
        Command::new("nix-shell")
            .arg("--run")
            .arg(build_cmd)
            .current_dir(client_dir)
            .output()
            .map_err(|e| format!("failed to run nix-shell: {e}"))?
    } else {
        println!("  Using direct g++ build...");
        Command::new("sh")
            .arg("-c")
            .arg(build_cmd)
            .current_dir(client_dir)
            .output()
            .map_err(|e| format!("failed to run g++: {e}"))?
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("build failed:\n{stderr}"));
    }

    Ok(())
}

fn run_cpp_decoder(client_dir: &Path, test_vectors_dir: &Path) -> Result<(), String> {
    let decoder_path = client_dir.join("decode_test_vectors");
    if !decoder_path.exists() {
        return Err("decode_test_vectors binary not found".to_string());
    }

    let output = Command::new(&decoder_path)
        .arg(test_vectors_dir)
        .current_dir(client_dir)
        .output()
        .map_err(|e| format!("failed to run decoder: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    print!("{stdout}");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("decoder failed:\n{stderr}"));
    }

    Ok(())
}

fn compare_bulk_metadata(dir: &str) -> Result<(), String> {
    let rust_path = Path::new(dir).join("bulk_root.json");
    let cpp_path = Path::new(dir).join("bulk_root_cpp.json");

    let rust_json = read_json(&rust_path)?;
    let cpp_json = read_json(&cpp_path)?;

    // Compare epoch.
    let rust_epoch = rust_json["epoch"].as_i64().ok_or("missing rust epoch")?;
    let cpp_epoch = cpp_json["epoch"].as_i64().ok_or("missing cpp epoch")?;
    compare_i64("epoch", rust_epoch, cpp_epoch)?;

    // Compare node_count.
    let rust_node_count = rust_json["node_count"]
        .as_i64()
        .ok_or("missing rust node_count")?;
    let cpp_node_count = cpp_json["node_count"]
        .as_i64()
        .ok_or("missing cpp node_count")?;
    compare_i64("node_count", rust_node_count, cpp_node_count)?;

    // Compare head_node_center.
    let rust_center = rust_json["head_node_center"]
        .as_array()
        .ok_or("missing rust head_node_center")?;
    let cpp_center = cpp_json["head_node_center"]
        .as_array()
        .ok_or("missing cpp head_node_center")?;
    compare_f64_array("head_node_center", rust_center, cpp_center, 1.0)?;

    // Compare meters_per_texel.
    let rust_mpt = rust_json["meters_per_texel"]
        .as_array()
        .ok_or("missing rust meters_per_texel")?;
    let cpp_mpt = cpp_json["meters_per_texel"]
        .as_array()
        .ok_or("missing cpp meters_per_texel")?;
    compare_f64_array("meters_per_texel", rust_mpt, cpp_mpt, 1.0)?;

    Ok(())
}

fn compare_node_data(dir: &str, node: &str) -> Result<(), String> {
    let rust_path = Path::new(dir).join(format!("node_{node}.json"));
    let cpp_path = Path::new(dir).join(format!("node_{node}_cpp.json"));

    let rust_json = read_json(&rust_path)?;
    let cpp_json = read_json(&cpp_path)?;

    // Compare mesh_count.
    let rust_mesh_count = rust_json["mesh_count"]
        .as_i64()
        .ok_or("missing rust mesh_count")?;
    let cpp_mesh_count = cpp_json["mesh_count"]
        .as_i64()
        .ok_or("missing cpp mesh_count")?;
    compare_i64("mesh_count", rust_mesh_count, cpp_mesh_count)?;

    // Compare each mesh.
    let rust_meshes = rust_json["meshes"]
        .as_array()
        .ok_or("missing rust meshes")?;
    let cpp_meshes = cpp_json["meshes"].as_array().ok_or("missing cpp meshes")?;

    if rust_meshes.len() != cpp_meshes.len() {
        return Err(format!(
            "mesh count mismatch: rust={}, cpp={}",
            rust_meshes.len(),
            cpp_meshes.len()
        ));
    }

    for (i, (rust_mesh, cpp_mesh)) in rust_meshes.iter().zip(cpp_meshes.iter()).enumerate() {
        compare_mesh(i, rust_mesh, cpp_mesh)?;
    }

    Ok(())
}

#[allow(clippy::similar_names)]
fn compare_mesh(
    idx: usize,
    rust_mesh: &serde_json::Value,
    cpp_mesh: &serde_json::Value,
) -> Result<(), String> {
    let prefix = format!("mesh[{idx}]");

    // Compare vertex_count.
    let rust_vertex_count = rust_mesh["vertex_count"]
        .as_i64()
        .ok_or(format!("{prefix}: missing rust vertex_count"))?;
    let cpp_vertex_count = cpp_mesh["vertex_count"]
        .as_i64()
        .ok_or(format!("{prefix}: missing cpp vertex_count"))?;
    compare_i64(
        &format!("{prefix}.vertex_count"),
        rust_vertex_count,
        cpp_vertex_count,
    )?;

    // Compare index_count.
    let rust_index_count = rust_mesh["index_count"]
        .as_i64()
        .ok_or(format!("{prefix}: missing rust index_count"))?;
    let cpp_index_count = cpp_mesh["index_count"]
        .as_i64()
        .ok_or(format!("{prefix}: missing cpp index_count"))?;
    compare_i64(
        &format!("{prefix}.index_count"),
        rust_index_count,
        cpp_index_count,
    )?;

    // Compare texture dimensions.
    let rust_tex_width = rust_mesh["texture_width"]
        .as_i64()
        .ok_or(format!("{prefix}: missing rust texture_width"))?;
    let cpp_tex_width = cpp_mesh["texture_width"]
        .as_i64()
        .ok_or(format!("{prefix}: missing cpp texture_width"))?;
    compare_i64(
        &format!("{prefix}.texture_width"),
        rust_tex_width,
        cpp_tex_width,
    )?;

    let rust_tex_height = rust_mesh["texture_height"]
        .as_i64()
        .ok_or(format!("{prefix}: missing rust texture_height"))?;
    let cpp_tex_height = cpp_mesh["texture_height"]
        .as_i64()
        .ok_or(format!("{prefix}: missing cpp texture_height"))?;
    compare_i64(
        &format!("{prefix}.texture_height"),
        rust_tex_height,
        cpp_tex_height,
    )?;

    // Compare uv_offset.
    let rust_uv_offset = rust_mesh["uv_offset"]
        .as_array()
        .ok_or(format!("{prefix}: missing rust uv_offset"))?;
    let cpp_uv_offset = cpp_mesh["uv_offset"]
        .as_array()
        .ok_or(format!("{prefix}: missing cpp uv_offset"))?;
    compare_f64_array(
        &format!("{prefix}.uv_offset"),
        rust_uv_offset,
        cpp_uv_offset,
        0.001,
    )?;

    // Compare uv_scale.
    let rust_uv_scale = rust_mesh["uv_scale"]
        .as_array()
        .ok_or(format!("{prefix}: missing rust uv_scale"))?;
    let cpp_uv_scale = cpp_mesh["uv_scale"]
        .as_array()
        .ok_or(format!("{prefix}: missing cpp uv_scale"))?;
    compare_f64_array(
        &format!("{prefix}.uv_scale"),
        rust_uv_scale,
        cpp_uv_scale,
        1e-9,
    )?;

    // Compare first_vertices.
    let rust_verts = rust_mesh["first_vertices"]
        .as_array()
        .ok_or(format!("{prefix}: missing rust first_vertices"))?;
    let cpp_verts = cpp_mesh["first_vertices"]
        .as_array()
        .ok_or(format!("{prefix}: missing cpp first_vertices"))?;
    compare_vertices(&format!("{prefix}.first_vertices"), rust_verts, cpp_verts)?;

    // Compare first_indices.
    let rust_indices = rust_mesh["first_indices"]
        .as_array()
        .ok_or(format!("{prefix}: missing rust first_indices"))?;
    let cpp_indices = cpp_mesh["first_indices"]
        .as_array()
        .ok_or(format!("{prefix}: missing cpp first_indices"))?;
    compare_i64_array(
        &format!("{prefix}.first_indices"),
        rust_indices,
        cpp_indices,
    )?;

    Ok(())
}

fn compare_vertices(
    name: &str,
    rust: &[serde_json::Value],
    cpp: &[serde_json::Value],
) -> Result<(), String> {
    if rust.len() != cpp.len() {
        return Err(format!(
            "{name}: length mismatch: rust={}, cpp={}",
            rust.len(),
            cpp.len()
        ));
    }

    for (i, (rust_vertex, cpp_vertex)) in rust.iter().zip(cpp.iter()).enumerate() {
        for field in &["x", "y", "z", "w", "u", "v"] {
            let rust_val = rust_vertex[*field]
                .as_i64()
                .ok_or(format!("{name}[{i}].{field}: missing rust value"))?;
            let cpp_val = cpp_vertex[*field]
                .as_i64()
                .ok_or(format!("{name}[{i}].{field}: missing cpp value"))?;
            if rust_val != cpp_val {
                return Err(format!(
                    "{name}[{i}].{field}: mismatch: rust={rust_val}, cpp={cpp_val}"
                ));
            }
        }
    }

    println!("  {name}: {len} vertices match", len = rust.len());
    Ok(())
}

fn compare_i64(name: &str, rust: i64, cpp: i64) -> Result<(), String> {
    if rust != cpp {
        return Err(format!("{name}: mismatch: rust={rust}, cpp={cpp}"));
    }
    println!("  {name}: {rust}");
    Ok(())
}

fn compare_i64_array(
    name: &str,
    rust: &[serde_json::Value],
    cpp: &[serde_json::Value],
) -> Result<(), String> {
    if rust.len() != cpp.len() {
        return Err(format!(
            "{name}: length mismatch: rust={}, cpp={}",
            rust.len(),
            cpp.len()
        ));
    }

    for (i, (rust_val, cpp_val)) in rust.iter().zip(cpp.iter()).enumerate() {
        let rust_num = rust_val
            .as_i64()
            .ok_or(format!("{name}[{i}]: invalid rust value"))?;
        let cpp_num = cpp_val
            .as_i64()
            .ok_or(format!("{name}[{i}]: invalid cpp value"))?;
        if rust_num != cpp_num {
            return Err(format!(
                "{name}[{i}]: mismatch: rust={rust_num}, cpp={cpp_num}"
            ));
        }
    }

    println!("  {name}: {len} values match", len = rust.len());
    Ok(())
}

fn compare_f64_array(
    name: &str,
    rust: &[serde_json::Value],
    cpp: &[serde_json::Value],
    tolerance: f64,
) -> Result<(), String> {
    if rust.len() != cpp.len() {
        return Err(format!(
            "{name}: length mismatch: rust={}, cpp={}",
            rust.len(),
            cpp.len()
        ));
    }

    for (i, (rust_val, cpp_val)) in rust.iter().zip(cpp.iter()).enumerate() {
        let rust_num = rust_val
            .as_f64()
            .ok_or(format!("{name}[{i}]: invalid rust value"))?;
        let cpp_num = cpp_val
            .as_f64()
            .ok_or(format!("{name}[{i}]: invalid cpp value"))?;
        let diff = (rust_num - cpp_num).abs();
        if diff > tolerance {
            return Err(format!(
                "{name}[{i}]: mismatch: rust={rust_num}, cpp={cpp_num}, diff={diff}"
            ));
        }
    }

    println!(
        "  {name}: {len} values match (tolerance={tolerance})",
        len = rust.len()
    );
    Ok(())
}

fn read_json(path: &Path) -> Result<serde_json::Value, String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    serde_json::from_str(&content).map_err(|e| format!("failed to parse {}: {e}", path.display()))
}

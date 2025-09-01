use {
    anyhow::Result,
    nargo::workspace::Workspace,
    nargo_cli::cli::compile_cmd::compile_workspace_full,
    nargo_toml::{resolve_workspace_from_toml, PackageSelection},
    noirc_driver::CompileOptions,
    provekit_common::NoirProofScheme,
    provekit_prover::NoirProofSchemeProver,
    provekit_r1cs_compiler::NoirProofSchemeBuilder,
    provekit_verifier::NoirProofSchemeVerifier,
    serde::Deserialize,
    std::path::Path,
    test_case::test_case,
};

#[derive(Debug, Deserialize)]
struct NargoToml {
    package: NargoTomlPackage,
}

#[derive(Debug, Deserialize)]
struct NargoTomlPackage {
    name: String,
}

fn test_compiler(test_case_path: impl AsRef<Path>) {
    let test_case_path = test_case_path.as_ref();

    compile_workspace(test_case_path).expect("Compiling workspace");

    let nargo_toml_path = test_case_path.join("Nargo.toml");

    let nargo_toml = std::fs::read_to_string(&nargo_toml_path).expect("Reading Nargo.toml");
    let nargo_toml: NargoToml = toml::from_str(&nargo_toml).expect("Deserializing Nargo.toml");

    let package_name = nargo_toml.package.name;

    let circuit_path = test_case_path.join(format!("target/{package_name}.json"));
    let witness_file_path = test_case_path.join("Prover.toml");

    let proof_schema = NoirProofScheme::from_file(&circuit_path).expect("Reading proof scheme");
    let input_map = proof_schema
        .read_witness(&witness_file_path)
        .expect("Reading witness data");

    let proof = proof_schema
        .prove(&input_map)
        .expect("While proving Noir program statement");

    proof_schema.verify(&proof).expect("Verifying proof");
}

pub fn compile_workspace(workspace_path: impl AsRef<Path>) -> Result<Workspace> {
    let workspace_path = workspace_path.as_ref();
    let workspace_path = if workspace_path.ends_with("Nargo.toml") {
        workspace_path.to_owned()
    } else {
        workspace_path.join("Nargo.toml")
    };

    // `resolve_workspace_from_toml` calls .normalize() under the hood which messes
    // up path resolution
    let workspace_path = workspace_path.canonicalize()?;

    let workspace =
        resolve_workspace_from_toml(&workspace_path, PackageSelection::DefaultOrAll, None)?;
    let compile_options = CompileOptions::default();

    compile_workspace_full(&workspace, &compile_options, None)?;

    Ok(workspace)
}

#[test_case("../../noir-examples/noir-r1cs-test-programs/acir_assert_zero")]
#[test_case("../../noir-examples/noir-r1cs-test-programs/simplest-read-only-memory")]
#[test_case("../../noir-examples/noir-r1cs-test-programs/read-only-memory")]
#[test_case("../../noir-examples/noir-r1cs-test-programs/range-check-u8")]
#[test_case("../../noir-examples/noir-r1cs-test-programs/range-check-u16")]
#[test_case("../../noir-examples/noir-r1cs-test-programs/range-check-mixed-bases")]
#[test_case("../../noir-examples/noir-r1cs-test-programs/read-write-memory")]
#[test_case("../../noir-examples/noir-r1cs-test-programs/conditional-write")]
#[test_case("../../noir-examples/noir-r1cs-test-programs/bin-opcode")]
#[test_case("../../noir-examples/noir-r1cs-test-programs/small-sha")]
#[test_case("../../noir-examples/noir-passport-examples/complete_age_check"; "complete_age_check")]
fn case(path: &str) {
    test_compiler(path);
}

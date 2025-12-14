#[cfg(not(feature = "include-data"))]
fn main() {}

#[cfg(feature = "include-data")]
fn main() {
    data_repo::init_repo();
}

#[cfg(feature = "include-data")]
mod data_repo {
    use std::{env, fs, path::PathBuf};

    use cargo_toml::Manifest;
    use git2::{Oid, Repository};
    use serde::Deserialize;

    #[derive(Clone, Debug, Deserialize)]
    struct Metadata {
        minecraft_data_owner: String,
        minecraft_data_repo: String,
    }

    pub fn init_repo() {
        println!("cargo:rerun-if-env-changed=MINECRAFT_DATA_REPO_PATH");
        println!("cargo:rerun-if-changed=Cargo.toml");

        let manifest = Manifest::<Metadata>::from_slice_with_metadata(include_bytes!("Cargo.toml"))
            .expect("Failed to read manifest (Cargo.toml)");
        let metadata = manifest
            .package
            .expect("missing package info in Cargo.toml")
            .metadata
            .expect("missing package.metadata in Cargo.toml");

        let repo_path = env::var("MINECRAFT_DATA_REPO_PATH")
            .map(PathBuf::from)
            .ok()
            .or_else(|| dirs::cache_dir().map(|p| p.join("minecraft-data")))
            .unwrap_or_else(|| PathBuf::from("minecraft-data"));

        println!(
            "cargo:rustc-env=MINECRAFT_DATA_PATH_INTERNAL={}",
            repo_path.to_string_lossy()
        );

        let version_oid: Oid;

        let repository_link = format!(
            "https://github.com/{}/{}.git",
            metadata.minecraft_data_owner, metadata.minecraft_data_repo
        );

        let repo = if repo_path.exists() {
            match Repository::open(&repo_path) {
                Ok(repo) => {
                    version_oid = repo.head().unwrap().target().unwrap();
                    if repo.find_commit(version_oid).is_ok() {
                        repo
                    } else {
                        // Drop the repo handle before deleting the directory
                        drop(repo);
                        fs::remove_dir_all(&repo_path)
                            .expect("could not delete existing repository");
                        Repository::clone(&repository_link, &repo_path)
                            .expect("failed to clone minecraft-data repo")
                    }
                }
                Err(_) => {
                    fs::remove_dir_all(&repo_path).expect("could not delete existing repository");
                    let repo = Repository::clone(&repository_link, &repo_path)
                        .expect("failed to clone minecraft-data repo");
                    version_oid = repo.head().unwrap().target().unwrap();
                    repo
                }
            }
        } else {
            let repo = Repository::clone(&repository_link, &repo_path)
                .expect("failed to clone minecraft-data repo");
            version_oid = repo.head().unwrap().target().unwrap();
            repo
        };

        repo.set_head_detached(version_oid)
            .expect("failed set head");

        let mut checkout = git2::build::CheckoutBuilder::new();
        checkout.force();
        repo.checkout_head(Some(&mut checkout))
            .expect("failed checkout index")
    }
}

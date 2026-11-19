//! The manifest of a Catalogue Talent, shared with the Swift parser (`docs/vecteurs-talents/`).

use murmure_talents_kit::{TalentManifestRule, TalentSource};

#[test]
fn the_shared_catalog_manifest_vector_is_accepted() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/vecteurs-talents/manifeste-catalogue-v1.json");
    let bytes = std::fs::read(path).expect("vector file");
    let manifest =
        TalentManifestRule::parse(&bytes, TalentSource::Catalog).expect("the shared vector");
    assert_eq!(manifest.id, "programme-de-sport");
    assert_eq!(manifest.author.github, ["thibault-ballof"]);
}

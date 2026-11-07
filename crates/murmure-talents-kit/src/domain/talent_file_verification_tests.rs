//! Contract: « Pour apprendre, les fichiers sont servis sans archive… » (bounds of the batch, exact size and hash).

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use super::fixtures::manifest;
use super::{
    TalentBundleRule, TalentFileVerification, TalentIndexError, TalentIndexFile, TalentKind,
    TalentOperationError,
};

fn file(path: &str, size: usize) -> TalentIndexFile {
    TalentIndexFile {
        path: path.to_owned(),
        size,
        sha256: "0".repeat(64),
    }
}

fn skill() -> TalentIndexFile {
    file("SKILL.md", 10)
}

fn valid(files: &[TalentIndexFile]) -> Result<(), TalentIndexError> {
    TalentFileVerification::validate(files)
}

fn files_of(count: usize) -> Vec<TalentIndexFile> {
    std::iter::once(skill())
        .chain((1..count).map(|index| file(&format!("annexe-{index}.md"), 1)))
        .collect()
}

#[test]
fn a_batch_has_one_to_32_distinct_files_with_a_skill_md() {
    assert_eq!(valid(&files_of(32)), Ok(()));
    assert_eq!(valid(&files_of(33)), Err(TalentIndexError::InvalidFiles));
    assert_eq!(valid(&[]), Err(TalentIndexError::InvalidFiles));
    assert_eq!(
        valid(&[skill(), skill()]),
        Err(TalentIndexError::InvalidFiles),
        "duplicate path"
    );
    assert_eq!(
        valid(&[file("README.md", 1)]),
        Err(TalentIndexError::InvalidFiles),
        "no SKILL.md"
    );
    assert_eq!(
        valid(&[
            file("skills/cuisine/SKILL.md", 1),
            file("skills/cuisine/recettes.md", 1)
        ]),
        Ok(()),
        "a batch of Skills"
    );
}

#[test]
fn a_batch_weighs_256_kib_at_most_in_all() {
    assert_eq!(valid(&[file("SKILL.md", 256 * 1024)]), Ok(()));
    assert_eq!(
        valid(&[file("SKILL.md", 256 * 1024 + 1)]),
        Err(TalentIndexError::InvalidFiles)
    );
    assert_eq!(
        valid(&[file("SKILL.md", 256 * 1024 - 10), file("a.md", 10)]),
        Ok(())
    );
    assert_eq!(
        valid(&[file("SKILL.md", 256 * 1024 - 10), file("a.md", 11)]),
        Err(TalentIndexError::InvalidFiles),
        "the sum, not each file"
    );
}

#[test]
fn a_path_has_eight_segments_at_most_and_no_trick() {
    let deep = |segments: usize| format!("skills/{}/SKILL.md", vec!["d"; segments - 2].join("/"));
    assert_eq!(valid(&[file(&deep(8), 1)]), Ok(()));
    assert_eq!(
        valid(&[file(&deep(9), 1)]),
        Err(TalentIndexError::InvalidFiles)
    );
    let forbidden = [
        "/SKILL.md",
        "",
        ".",
        "a/../b",
        "a/./b",
        "a//b",
        "a\\b",
        "a%2Fb",
        "a?b",
        "a#b",
        "a:b",
        "a\tb",
        "dossier/",
        "../SKILL.md",
    ];
    for path in forbidden {
        assert_eq!(
            valid(&[skill(), file(path, 1)]),
            Err(TalentIndexError::InvalidFiles),
            "{path:?}"
        );
    }
    assert_eq!(
        valid(&[TalentIndexFile {
            sha256: "A".repeat(64),
            ..skill()
        }]),
        Err(TalentIndexError::InvalidFiles),
        "uppercase hash"
    );
    assert_eq!(
        valid(&[TalentIndexFile {
            sha256: "0".repeat(63),
            ..skill()
        }]),
        Err(TalentIndexError::InvalidFiles),
        "short hash"
    );
}

fn downloaded(entries: &[(&str, &str)]) -> BTreeMap<String, Vec<u8>> {
    entries
        .iter()
        .map(|(path, content)| ((*path).to_owned(), content.as_bytes().to_vec()))
        .collect()
}

fn described(path: &str, content: &str) -> TalentIndexFile {
    TalentIndexFile {
        path: path.to_owned(),
        size: content.len(),
        sha256: format!("{:x}", Sha256::digest(content.as_bytes())),
    }
}

#[test]
fn what_is_downloaded_matches_the_index_byte_for_byte_or_the_whole_batch_is_refused() {
    let files = [described("SKILL.md", "corps"), described("a.md", "annexe")];
    assert_eq!(
        TalentFileVerification::verify(
            &files,
            &downloaded(&[("SKILL.md", "corps"), ("a.md", "annexe")])
        ),
        Ok(())
    );
    assert_eq!(
        TalentFileVerification::verify(
            &files,
            &downloaded(&[("SKILL.md", "corps"), ("a.md", "annexé")])
        ),
        Err(TalentIndexError::InvalidFiles),
        "another hash"
    );
    assert_eq!(
        TalentFileVerification::verify(
            &files,
            &downloaded(&[("SKILL.md", "corps"), ("a.md", "annexe!")])
        ),
        Err(TalentIndexError::InvalidFiles),
        "another size"
    );
    assert_eq!(
        TalentFileVerification::verify(&files, &downloaded(&[("SKILL.md", "corps")])),
        Err(TalentIndexError::InvalidFiles),
        "a file missing"
    );
    assert_eq!(
        TalentFileVerification::verify(
            &files,
            &downloaded(&[("SKILL.md", "corps"), ("a.md", "annexe"), ("b.md", "")])
        ),
        Err(TalentIndexError::InvalidFiles),
        "a file more"
    );
}

#[test]
fn the_single_skill_format_makes_one_skill_named_like_the_talent_and_the_batch_format_one_per_folder(
) {
    let talent = manifest("coach-cuisine", "1.0.0", &[], &[]);
    let single = TalentBundleRule::group_skills(
        &talent,
        &downloaded(&[("SKILL.md", "corps"), ("recettes/a.md", "x")]),
    )
    .expect("one Skill");
    assert_eq!(single.keys().collect::<Vec<_>>(), ["coach-cuisine"]);
    assert_eq!(
        single["coach-cuisine"]
            .iter()
            .map(|(path, _)| path.as_str())
            .collect::<Vec<_>>(),
        ["SKILL.md", "recettes/a.md"]
    );
    let batch = TalentBundleRule::group_skills(
        &talent,
        &downloaded(&[
            ("skills/a-skill/SKILL.md", "1"),
            ("skills/b-skill/SKILL.md", "2"),
            ("skills/b-skill/notes.md", "3"),
        ]),
    )
    .expect("two Skills");
    assert_eq!(batch.keys().collect::<Vec<_>>(), ["a-skill", "b-skill"]);
    assert_eq!(
        batch["b-skill"]
            .iter()
            .map(|(path, _)| path.as_str())
            .collect::<Vec<_>>(),
        ["SKILL.md", "notes.md"]
    );
}

#[test]
fn a_file_that_is_not_utf8_a_skill_folder_with_a_bad_name_and_a_talent_with_code_are_invalid_resources(
) {
    let talent = manifest("coach-cuisine", "1.0.0", &[], &[]);
    let refused = |result: Result<_, TalentOperationError>| {
        assert_eq!(
            result.map(|_: BTreeMap<String, super::SkillFiles>| ()),
            Err(TalentOperationError::InvalidResources)
        )
    };
    let mut binary = downloaded(&[("SKILL.md", "corps")]);
    binary.insert("annexe.bin".to_owned(), vec![0xff, 0xfe, 0x00]);
    refused(TalentBundleRule::group_skills(&talent, &binary));
    refused(TalentBundleRule::group_skills(
        &talent,
        &downloaded(&[("skills/Mauvais_Nom/SKILL.md", "1")]),
    ));
    refused(TalentBundleRule::group_skills(
        &talent,
        &downloaded(&[("skills/SKILL.md", "1")]),
    ));
    let mut with_code = talent.clone();
    with_code.kind = TalentKind::WithCode;
    refused(TalentBundleRule::group_skills(
        &with_code,
        &downloaded(&[("SKILL.md", "corps")]),
    ));
}

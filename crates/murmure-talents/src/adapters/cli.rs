//! `murmure-talents validate <root>` · `convert <source> [--id <id>] [--output <new folder>] [--json]` · `build-index <root> --output <new folder> --base-url <url> --sequence <n>` ·
//! `sign <root> --index <index.json> --key <0600 file outside the repository>` ·
//! `delegate --root-key <file> --publication-public-key <base64> --sequence <n> --valid-days <1…400> --output <new folder>` ·
//! `publication-key --public <file>` ·
//! `publish <root> --output <new folder> --base-url <url> --sequence <n|auto> --key <file> --delegation <folder>
//! --root-public-key <base64>`. Exit 0 on success, 1 otherwise; an error is `ERREUR : …` on stderr and never
//! carries key material.

use std::path::{Path, PathBuf};

use clap::{Arg, ArgAction, ArgMatches, Command};
use murmure_talents_kit::{TalentBuiltInResource, TalentManifest, BUILT_IN_MANIFESTS};

use super::FsCatalogDisk;
use crate::application::{
    build_index, codeowners, convert_source, delegate, DelegateRequest, diff_catalogs, publication_public_key, publish_catalog, sign_index, validate_catalog,
    PublishRequest,
};
use crate::domain::CatalogError;

pub struct CliResult {
    pub exit_code: i32,
    pub standard_output: String,
    pub standard_error: String,
}

pub const USAGE: &str = "Usage : murmure-talents validate <racine>\n        murmure-talents convert <source> [--id <identifiant>] [--output <dossier neuf>] [--json]\n        murmure-talents codeowners <racine>\n        murmure-talents diff <racine-base> <racine-tête> [--json]\n        murmure-talents build-index <racine> --output <dossier neuf> --base-url <url> --sequence <entier>\n        murmure-talents sign <racine> --index <index.json> --key <fichier 0600 hors dépôt>\n        murmure-talents delegate --root-key <fichier 0600 hors dépôt> --publication-public-key <base64> --sequence <entier> --valid-days <1 à 400> --output <dossier neuf>\n        murmure-talents publication-key --public <fichier 0600 hors dépôt>\n        murmure-talents publish <racine> --output <dossier neuf> --base-url <url> --sequence <entier|auto> --key <fichier 0600 hors dépôt> --delegation <dossier> --root-public-key <base64>\nLa racine est obligatoire. convert lit un dossier local sans réseau ni lien et propose des Talents à relire (TODO-fr bloque validate) ; sortie 1 quand rien n’est convertible. build-index écrit dans --output ; sign écrit <index>.sig sans modifier l’index. delegate est un geste humain hors ligne, jamais en CI. publish enchaîne validate, build-index, sign et la copie de la délégation, puis vérifie la chaîne avec le code du Relais ; --sequence auto vaut l’horodatage UTC en secondes.\n";

fn command() -> Command {
    let root = || Arg::new("root").required(true).allow_hyphen_values(true);
    let option = |name: &'static str| {
        Arg::new(name).long(name).required(true).action(ArgAction::Set).num_args(1).allow_hyphen_values(true)
    };
    Command::new("murmure-talents")
        .disable_help_flag(true)
        .disable_version_flag(true)
        .disable_help_subcommand(true)
        .subcommand_required(true)
        .subcommand(Command::new("validate").disable_help_flag(true).arg(root()))
        .subcommand(
            Command::new("convert")
                .disable_help_flag(true)
                .arg(Arg::new("source").required(true).allow_hyphen_values(true))
                .arg(Arg::new("id").long("id").action(ArgAction::Set).num_args(1).allow_hyphen_values(true))
                .arg(Arg::new("output").long("output").action(ArgAction::Set).num_args(1).allow_hyphen_values(true))
                .arg(Arg::new("json").long("json").action(ArgAction::SetTrue)),
        )
        .subcommand(Command::new("codeowners").disable_help_flag(true).arg(root()))
        .subcommand(
            Command::new("diff")
                .disable_help_flag(true)
                .arg(Arg::new("base").required(true).allow_hyphen_values(true))
                .arg(Arg::new("head").required(true).allow_hyphen_values(true))
                .arg(Arg::new("json").long("json").action(ArgAction::SetTrue)),
        )
        .subcommand(
            Command::new("build-index")
                .disable_help_flag(true)
                .arg(root())
                .arg(option("output"))
                .arg(option("base-url"))
                .arg(option("sequence")),
        )
        .subcommand(Command::new("sign").disable_help_flag(true).arg(root()).arg(option("index")).arg(option("key")))
        .subcommand(
            Command::new("delegate")
                .disable_help_flag(true)
                .arg(option("root-key"))
                .arg(option("publication-public-key"))
                .arg(option("sequence"))
                .arg(option("valid-days"))
                .arg(option("output")),
        )
        .subcommand(Command::new("publication-key").disable_help_flag(true).arg(option("public")))
        .subcommand(
            Command::new("publish")
                .disable_help_flag(true)
                .arg(root())
                .arg(option("output"))
                .arg(option("base-url"))
                .arg(option("sequence"))
                .arg(option("key"))
                .arg(option("delegation"))
                .arg(option("root-public-key")),
        )
}

fn value(matches: &ArgMatches, name: &str) -> String {
    matches.get_one::<String>(name).cloned().unwrap_or_default()
}

fn built_in_manifests() -> Result<Vec<TalentManifest>, CatalogError> {
    BUILT_IN_MANIFESTS
        .iter()
        .map(|text| {
            TalentBuiltInResource::parse(text.as_bytes())
                .map(|resource| resource.manifest)
                .map_err(|_| CatalogError::new("Manifestes embarqués invalides."))
        })
        .collect()
}

fn absolute(path: &str) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| PathBuf::from(path))
}

fn execute(arguments: &[String], now: i64) -> Result<CliResult, CatalogError> {
    let usage = || CatalogError::new(USAGE.trim_end());
    let matches = command().try_get_matches_from(std::iter::once("murmure-talents".to_owned()).chain(arguments.iter().cloned()))
        .map_err(|_| usage())?;
    let (name, sub) = matches.subcommand().ok_or_else(usage)?;
    let disk = FsCatalogDisk;
    if name == "convert" {
        let output = sub.get_one::<String>("output").map(|path| absolute(path));
        let report = convert_source(
            &disk,
            &FsCatalogDisk::resolve(Path::new(&value(sub, "source"))),
            sub.get_one::<String>("id").map(String::as_str),
            output.as_deref(),
        )?;
        let text = if sub.get_flag("json") { report.json() } else { report.text() };
        return Ok(CliResult { exit_code: i32::from(report.talents.is_empty()), standard_output: text, standard_error: String::new() });
    }
    let success = |text: String| CliResult { exit_code: 0, standard_output: text, standard_error: String::new() };
    if name == "diff" {
        let report = diff_catalogs(
            &disk,
            &FsCatalogDisk::resolve(Path::new(&value(sub, "base"))),
            &FsCatalogDisk::resolve(Path::new(&value(sub, "head"))),
        )?;
        let text = if sub.get_flag("json") { report.json() } else { report.text() };
        return Ok(CliResult { exit_code: i32::from(!report.is_valid()), standard_output: text, standard_error: String::new() });
    }
    if name == "delegate" || name == "publication-key" {
        // The key must be outside the repository the human works in.
        let repository = FsCatalogDisk::resolve(Path::new("."));
        if name == "publication-key" {
            let printed = publication_public_key(&disk, &absolute(&value(sub, "public")), &repository)?;
            return Ok(success(printed + "\n"));
        }
        let invalid_number = || CatalogError::new("Séquence ou nombre de jours invalide.");
        let request = DelegateRequest {
            root_key: &absolute(&value(sub, "root-key")),
            repository: &repository,
            publication_public_key: &value(sub, "publication-public-key"),
            sequence: value(sub, "sequence").parse().map_err(|_| invalid_number())?,
            valid_days: value(sub, "valid-days").parse().map_err(|_| invalid_number())?,
            output: &absolute(&value(sub, "output")),
        };
        delegate(&disk, &request, now)?;
        return Ok(success("Délégation signée : delegation.json et delegation.json.sig écrits.\n".to_owned()));
    }
    let root = FsCatalogDisk::resolve(Path::new(&value(sub, "root")));
    if name == "publish" {
        let request = PublishRequest {
            root: &root,
            output: &absolute(&value(sub, "output")),
            base_url: &value(sub, "base-url"),
            sequence: &value(sub, "sequence"),
            key: &absolute(&value(sub, "key")),
            delegation: &absolute(&value(sub, "delegation")),
            root_public_key: &value(sub, "root-public-key"),
        };
        let report = publish_catalog(&disk, &request, now, &built_in_manifests()?)?;
        if !report.is_valid() {
            return Ok(CliResult { exit_code: 1, standard_output: report.text(), standard_error: String::new() });
        }
        return Ok(success(report.text() + "Publication prête : chaîne de confiance vérifiée.\n"));
    }
    if name == "codeowners" {
        return Ok(success(codeowners(&disk, &root)?));
    }
    match name {
        "validate" | "build-index" => {
            let built_in = built_in_manifests()?;
            let report = if name == "validate" {
                validate_catalog(&disk, &root, now, &built_in)?
            } else {
                let output = absolute(&value(sub, "output"));
                build_index(&disk, &root, &output, &value(sub, "base-url"), &value(sub, "sequence"), now, &built_in)?
            };
            if !report.is_valid() {
                return Ok(CliResult { exit_code: 1, standard_output: report.text(), standard_error: String::new() });
            }
            let closing = if name == "build-index" { "Index et fichiers prêts pour signature.\n" } else { "" };
            Ok(success(report.text() + closing))
        }
        _ => {
            sign_index(&disk, &root, &absolute(&value(sub, "index")), &absolute(&value(sub, "key")))?;
            Ok(success("Signature détachée écrite.\n".to_owned()))
        }
    }
}

/// Parses the arguments (clap), runs the use case on the real disk with the built-in manifests of the Kit.
pub fn run(arguments: &[String], now: i64) -> CliResult {
    execute(arguments, now).unwrap_or_else(|error| CliResult {
        exit_code: 1,
        standard_output: String::new(),
        standard_error: format!("ERREUR : {}\n", error.0),
    })
}

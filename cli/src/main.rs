use anyhow::{Context, Result};
use directories::{UserDirs, ProjectDirs};
use scrapper::{EchaSite, Section, Subsection};
use std::fs;
use std::borrow::{Borrow, Cow};
use std::path::{Path, PathBuf};
use scrapper::Subsection::{Boundary, Other};
use serde::{Deserialize, Serialize};

fn main() -> Result<()> {
    // **************************************************
    // ************ CLI args requirements ***************
    // **************************************************
    let url : Cow<'static, str> = match std::env::args().nth(1) {
        Some(url) => Cow::from(url),
        None => {
            println!("No CLI URL provided, using default.");
            Cow::from("https://echa.europa.eu/registration-dossier/-/registered-dossier/24529")
            //"https://echa.europa.eu/registration-dossier/-/registered-dossier/26453".into()
        }
    };

    // **************************************************
    // ************ Create folder & files ***************
    // **************************************************
    let mut ofile_path = PathBuf::new();
    if let Some(user_dirs) = UserDirs::new() {
        let output_dir = user_dirs
            .desktop_dir()
            .map(|path| path.join("echars_output"))
            .context("Couldn't create output folder path")?;

        // Output file name is the number of the dossier in the url.
        // The file is truncated if it already exists.
        let dossier = url
            .split('/')
            .last()
            .expect("Couldn't obtain dossier number");

        ofile_path = output_dir.join(dossier).with_extension("tsv");

        fs::create_dir_all(output_dir)?; //.expect("Couldn't create output folder path");
    }

    // **************************************************
    // **************** Getting data ********************
    // **************************************************
    let mut echa = EchaSite::new(url.borrow());
    let identification = echa.get_constituents(Section::Identification);
    let boundary = echa.get_constituents(Section::Composition(Subsection::Boundary));
    let legal = echa.get_constituents(Section::Composition(Subsection::LegalEntity));
    let generated = echa.get_constituents(Section::Composition(Subsection::Generated));
    let other = echa.get_constituents(Section::Composition(Subsection::Other));

    // **************************************************
    // ****************** Save data *********************
    // **************************************************
    let mut wtr = csv::WriterBuilder::new()
        .delimiter(b'\t')
        .from_path(ofile_path)?;

    for data in identification {
        wtr.serialize(data)?;
    }

    for data in boundary {
        wtr.serialize(data)?;
    }

    for data in legal {
        wtr.serialize(data)?;
    }

    for data in generated {
        wtr.serialize(data)?;
    }

    for data in other {
        wtr.serialize(data)?;
    }

    Ok(())
}

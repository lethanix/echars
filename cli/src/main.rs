use anyhow::{Context, Result};
use directories::UserDirs;
use scrapper::data_from;
use scrapper::Section;
use scrapper::Subsection;
use std::fs;

fn main() -> Result<()> {
    // **************************************************
    // ************ CLI args requirements ***************
    // **************************************************
    let url = match std::env::args().nth(1) {
        Some(url) => url,
        None => {
            println!("No CLI URL provided, using default.");
            "https://echa.europa.eu/registration-dossier/-/registered-dossier/24529".into()
            //"https://echa.europa.eu/registration-dossier/-/registered-dossier/26453".into()
        }
    };

    // **************************************************
    // ************ Create folder & files ***************
    // **************************************************
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

        let _ofile_path = output_dir.join(dossier).with_extension("tsv");

        fs::create_dir_all(output_dir)?; //.expect("Couldn't create output folder path");
    }

    // **************************************************
    // **************** Getting data ********************
    // **************************************************
    //let _identification = data_from(&url, Section::Identification)?;
    let boundary = data_from(&url, Section::Composition(Subsection::Boundary))?;
    //let _legal = data_from(&url, Section::Composition(Subsection::LegalEntity))?;
    //let generated = data_from(&url, Section::Composition(Subsection::Generated))?;
    //let _other = data_from(&url, Section::Composition(Subsection::Other))?;

    println!("{:#?}", boundary);

    Ok(())
}

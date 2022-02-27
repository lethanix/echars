use anyhow::{Context, Result};
use directories::{UserDirs, ProjectDirs};
use scrapper::{EchaSite, Section, Subsection};
use std::fs;
use std::borrow::{Borrow, Cow};
use std::path::{Path, PathBuf};
use scrapper::Subsection::Boundary;
use serde::{Deserialize, Serialize};

// idcoordinates2D	FragFp	EC	Weblink	Structure	Section	Image	Subsection	Name	Reference Substance	Constitute	Reference EC	Reference CAS
#[derive(Debug, Deserialize, Serialize)]
struct Record {
    #[serde(skip)]
    idcoordinates2D: String,
    #[serde(skip)]
    FragFp: String,
    #[serde(alias="EC")]
    id: String,
    #[serde(alias="Weblink")]
    weblink: String,
    #[serde(skip)]
    structure: String,
    #[serde(alias="Section")]
    section: String,
    #[serde(alias="Image")]
    image: String,
    #[serde(alias="Subsection")]
    subsection: String,
    #[serde(alias="Name")]
    name: String,
    #[serde(alias="Reference Substance", rename(serialize="Reference Substance"))]
    substance: String,
    #[serde(alias="Constitute")]
    constitute: String,
    #[serde(alias="Reference EC", rename(serialize="Reference EC"))]
    ec: String,
    #[serde(alias="Reference CAS", rename(serialize="Reference CAS"))]
    cas: String,
}

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
    let _legal = echa.get_constituents(Section::Composition(Subsection::LegalEntity));
    let _boundary = echa.get_constituents(Section::Composition(Boundary));

    println!("{:#?}", identification);

    // **************************************************
    // ****************** Save data *********************
    // **************************************************
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .comment(Some(b'<')) // Ignore datawarrior extra info
        .from_path(Path::new("Templado.dwar"))?;
    let headers = rdr.headers()?;

    // for result in rdr.deserialize() {
    //     let record: Record = result?;
    //     println!("{:?}", record);
    //     // Try this if you don't like each record smushed on one line:
    //     // println!("{:#?}", record);
    // }

    let mut wtr = csv::WriterBuilder::new()
        .delimiter(b'\t')
        .from_path(ofile_path)?;

    // idcoordinates2D	FragFp	EC	Weblink	Structure	Section	Image	Subsection	Name	Reference Substance	Constitute	Reference EC	Reference CAS
    // wtr.write_record(headers)?;

    for data in identification {
        wtr.serialize(Record {
            idcoordinates2D: "".to_string(),
            FragFp: "".to_string(),
            id: "1".to_string(),
            weblink: url.to_string(),
            structure: "".to_string(),
            section: "Identification".to_string(),
            image: data.get("Image link").unwrap().clone(),
            subsection: "".to_string(),
            name: data.get("Display Name").unwrap().clone(),
            substance: data.get("Display Name").unwrap().clone(),
            constitute: data.get("Constituent").unwrap_or(&"".to_string()).clone(),
            ec: data.get("EC Number").unwrap().clone(),
            cas: data.get("CAS Number").unwrap().clone()
        })?;
    }

    // identification.iter().for_each(|data| {
    //     wtr.serialize(Record {
    //         idcoordinates2D: "".to_string(),
    //         FragFp: "".to_string(),
    //         id: "1".to_string(),
    //         weblink: url.to_string(),
    //         structure: "".to_string(),
    //         section: "Identification".to_string(),
    //         image: data.get("Image link").unwrap().clone(),
    //         subsection: "".to_string(),
    //         name: data.get("Display Name").unwrap().clone(),
    //         substance: data.get("Display Name").unwrap().clone(),
    //         constitute: data.get("Constituent").unwrap().clone(),
    //         ec: data.get("EC Number").unwrap().clone(),
    //         cas: data.get("CAS Number").unwrap().clone()
    //     })?;
    // });


    Ok(())
}

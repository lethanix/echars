use anyhow::{anyhow, Context, Result};
use directories::{ProjectDirs, UserDirs};
use oxychem::{get_cas, get_cid, search_formula};
use scrapper::Subsection::{Boundary, Other};
use scrapper::{EchaSite, Section, Subsection};
use serde::{Deserialize, Serialize};
use std::borrow::{Borrow, Cow};
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    // **************************************************
    // ************ CLI args requirements ***************
    // **************************************************
    let url: Cow<'static, str> = match std::env::args().nth(1) {
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
        // ! TODO: Compare cas with pubchem data and retrieve sdf file.
        // let formula = data.formula.clone();
        // let list = match search_formula(&formula) {
        //     Ok(it) => it,
        //     Err(_err) => {//
        //         return Err(anyhow!(//
        //             "Couldn't obtain list of cids fr// om molecular formula -> {_err}"
        //         ))//
        //     }//
        // };//

        // //let cid = get_cid(data.substance.clone()).// unwrap_or(0);
        // let cas_list: Vec<String> = list//
        //     .iter()//
        //     .map(|cid| cid.parse::<isize>().expect("// Couldn't parse cid to isize"))
        //     .map(|cid| get_cas(cid).unwrap_or("N/A".// to_string()))
        //     .collect();//

        // let w = cas_list.iter()//
        //     .enumerate()//
        //     .scan(0, |state, (idx, value)| {//
        //         if value == &data.cas {//
        //             *state = idx;//
        //         }//
        //         Some(*state)//
        //     });//

        // dbg!(w);//

        // //let cas = get_cas(cid).unwrap_or("N/A".to_// string());
        // eprintln!("list = {:#?}", list);
        // eprintln!("CAS\n\tPubchem: {:?}\n\tEcha: {:?}", cas_list, &data.cas);
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
